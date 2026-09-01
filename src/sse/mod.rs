use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::Stream;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// SSE message sent to connected clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SseMessage {
    pub channel: String,
    pub event: String,
    pub data: serde_json::Value,
    pub id: Option<String>,
}

/// Central SSE broadcast hub with per-channel tokio broadcast channels.
#[derive(Clone)]
pub struct SseBroadcastHub {
    channels: Arc<Mutex<HashMap<String, broadcast::Sender<SseMessage>>>>,
    capacity: usize,
}

impl SseBroadcastHub {
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
            capacity,
        }
    }

    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<SseMessage> {
        let mut channels = self.channels.lock().unwrap();
        if let Some(sender) = channels.get(channel) {
            sender.subscribe()
        } else {
            let (sender, receiver) = broadcast::channel(self.capacity);
            channels.insert(channel.to_string(), sender);
            receiver
        }
    }

    pub fn broadcast(&self, msg: SseMessage) -> Result<usize, String> {
        let channels = self.channels.lock().unwrap();
        if let Some(sender) = channels.get(&msg.channel) {
            sender.send(msg).map_err(|e| e.to_string())
        } else {
            Ok(0) // No subscribers
        }
    }

    /// Creates an Axum SSE stream handler for a given channel.
    /// Includes automatic 30s heartbeat keep-alives.
    pub fn sse_stream(&self, channel: &str) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        let receiver = self.subscribe(channel);
        let stream = BroadcastStream::new(receiver).filter_map(|res| {
            match res {
                Ok(msg) => {
                    let mut event = Event::default()
                        .event(msg.event)
                        .json_data(msg.data)
                        .unwrap_or_else(|_| Event::default().data(""));
                    if let Some(id) = msg.id {
                        event = event.id(id);
                    }
                    Some(Ok(event))
                }
                Err(_) => {
                    // Receiver lagged, log or ignore
                    None
                }
            }
        });

        Sse::new(stream).keep_alive(KeepAlive::default())
    }
}
