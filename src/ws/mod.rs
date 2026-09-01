use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Generic message envelope for real-time collaboration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub room: String,
    pub event: String,
    pub sender_token: Option<String>,
    pub payload: serde_json::Value,
}

/// Central pub/sub broadcasting hub for WebSocket collaboration rooms.
#[derive(Clone)]
pub struct BroadcastHub {
    rooms: Arc<Mutex<HashMap<String, broadcast::Sender<WsMessage>>>>,
    capacity: usize,
}

impl Default for BroadcastHub {
    fn default() -> Self {
        Self::new(128)
    }
}

impl BroadcastHub {
    /// Creates a new broadcast hub with a per-room channel buffer capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            rooms: Arc::new(Mutex::new(HashMap::new())),
            capacity,
        }
    }

    /// Subscribes to or creates a room's broadcast receiver.
    pub fn subscribe(&self, room: &str) -> broadcast::Receiver<WsMessage> {
        let mut rooms = self.rooms.lock().unwrap();
        let sender = rooms.entry(room.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(self.capacity);
            tx
        });
        sender.subscribe()
    }

    /// Broadcasts a message to all active clients in a room.
    pub fn broadcast(&self, msg: WsMessage) -> Result<usize, String> {
        let rooms = self.rooms.lock().unwrap();
        if let Some(sender) = rooms.get(&msg.room) {
            sender.send(msg).map_err(|e| e.to_string())
        } else {
            Ok(0)
        }
    }

    /// Handles an incoming WebSocket connection, bridging it with the room pub/sub channel.
    pub async fn handle_socket(
        self: Arc<Self>,
        socket: WebSocket,
        room: String,
        user_token: Option<String>,
    ) {
        let (mut sender, mut receiver) = socket.split();
        let mut room_rx = self.subscribe(&room);

        // Task to forward room broadcasts to the WebSocket client
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = room_rx.recv().await {
                if let Ok(json_text) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(json_text)).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Task to receive client events and broadcast them to the room
        let hub_clone = self.clone();
        let room_clone = room.clone();
        let user_token_clone = user_token.clone();

        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(mut incoming) = serde_json::from_str::<WsMessage>(&text) {
                        incoming.room = room_clone.clone();
                        incoming.sender_token = user_token_clone.clone();
                        let _ = hub_clone.broadcast(incoming);
                    }
                }
            }
        });

        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_hub_pubsub() {
        let hub = BroadcastHub::new(16);
        let mut rx1 = hub.subscribe("trip-101");
        let mut rx2 = hub.subscribe("trip-101");
        let mut rx_other = hub.subscribe("trip-202");

        let msg = WsMessage {
            room: "trip-101".to_string(),
            event: "pin_added".to_string(),
            sender_token: Some("usr_alice".to_string()),
            payload: serde_json::json!({ "title": "Tokyo Tower" }),
        };

        let delivered = hub.broadcast(msg.clone()).expect("broadcast ok");
        assert_eq!(delivered, 2);

        let received1 = rx1.recv().await.expect("recv 1");
        assert_eq!(received1.event, "pin_added");
        assert_eq!(received1.payload["title"], "Tokyo Tower");

        let received2 = rx2.recv().await.expect("recv 2");
        assert_eq!(received2.sender_token.as_deref(), Some("usr_alice"));

        // rx_other should not receive messages for trip-101
        assert!(rx_other.try_recv().is_err());
    }
}
