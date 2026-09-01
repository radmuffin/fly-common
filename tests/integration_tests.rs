use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use fly_common::prelude::*;
use http_body_util::BodyExt;
use serde_json::Value;
use std::net::{Ipv4Addr, Ipv6Addr};
use tower::ServiceExt;

#[tokio::test]
async fn test_user_token_auth_extractor() {
    let app = Router::new().route(
        "/profile",
        get(|user_token: UserToken| async move { format!("token:{}", user_token.as_str()) }),
    );

    // 1. With X-User-Token header
    let req = Request::builder()
        .uri("/profile")
        .header("x-user-token", "usr_alpha_123-456")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"token:usr_alpha_123-456");

    // 2. With query string token
    let req_q = Request::builder()
        .uri("/profile?user_token=query_token_789")
        .body(Body::empty())
        .unwrap();
    let res_q = app.clone().oneshot(req_q).await.unwrap();
    assert_eq!(res_q.status(), StatusCode::OK);
    let body_q = res_q.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body_q[..], b"token:query_token_789");

    // 3. Sanitization of invalid characters
    let dirty_token = UserToken::new("valid-token_123!@#$%^&*()");
    assert_eq!(dirty_token.as_str(), "valid-token_123");
    assert!(dirty_token.is_present());

    let empty_token = UserToken::new("");
    assert_eq!(empty_token.as_str(), "");
    assert!(!empty_token.is_present());
}

#[test]
fn test_fly_db_pooling_and_transactions() {
    // 1. In-memory single connection
    let mut conn = FlyDb::open_in_memory().expect("open memory");
    FlyDb::run_migrations(
        &mut conn,
        &[
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
            "INSERT INTO users (name) VALUES ('Alice');",
            "INSERT INTO users (name) VALUES ('Bob');",
        ],
    )
    .expect("migrations");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 2);

    // 2. Shared connection pool
    let pool = FlyDb::open_shared_in_memory().expect("open shared memory");
    {
        let conn_locked = pool.lock().unwrap();
        let wal: String = conn_locked
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap_or_default();
        // Memory DBs use memory journal mode, but foreign keys are active
        let fk: i64 = conn_locked
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("fk");
        assert_eq!(fk, 1);
        assert!(!wal.is_empty());
    }

    // 3. Failed migration rollback
    let rollback_res = FlyDb::run_migrations(
        &mut conn,
        &[
            "INSERT INTO users (name) VALUES ('Charlie');",
            "INVALID SQL STATEMENT TRIGGERING ERROR;",
        ],
    );
    assert!(rollback_res.is_err());

    let count_after_fail: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
        .expect("count after fail");
    // Charlie was rolled back
    assert_eq!(count_after_fail, 2);
}

#[test]
fn test_models_serialization_and_responses() {
    // ApiResponse::ok
    let ok_resp = ApiResponse::ok(vec!["Tokyo", "Kyoto", "Osaka"]);
    assert!(ok_resp.success);
    assert_eq!(ok_resp.data.as_ref().unwrap().len(), 3);
    assert_eq!(ok_resp.error, None);

    // ApiResponse::err
    let err_resp: ApiResponse<String> = ApiResponse::err("Unauthorized access");
    assert!(!err_resp.success);
    assert_eq!(err_resp.data, None);
    assert_eq!(err_resp.error.as_deref(), Some("Unauthorized access"));

    // ApiResponse::message
    let msg_resp: ApiResponse<()> = ApiResponse::message("Operation completed");
    assert!(msg_resp.success);
    assert_eq!(msg_resp.message.as_deref(), Some("Operation completed"));

    // UserProfile and CollaboratorProfile models
    let profile = UserProfile {
        user_token: "usr_xyz".to_string(),
        name: "Alex".to_string(),
        avatar: "🦊".to_string(),
        color: "#f97316".to_string(),
    };
    let json_str = serde_json::to_string(&profile).expect("serialize profile");
    let deserialized: UserProfile = serde_json::from_str(&json_str).expect("deserialize");
    assert_eq!(profile, deserialized);

    let collab = CollaboratorProfile {
        name: "Sam".to_string(),
        avatar: "🧭".to_string(),
        color: "#3b82f6".to_string(),
        is_owner: true,
    };
    let collab_json = serde_json::to_string(&collab).expect("serialize collab");
    let collab_de: CollaboratorProfile =
        serde_json::from_str(&collab_json).expect("deserialize collab");
    assert_eq!(collab, collab_de);
}

#[test]
fn test_security_ssrf_comprehensive_matrix() {
    // IPv4 Restricted Range Matrix
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(127, 0, 0, 1).into()
    )); // Loopback
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(10, 0, 1, 50).into()
    )); // RFC 1918 Private
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(172, 16, 0, 1).into()
    )); // RFC 1918 172.16
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(172, 31, 255, 255).into()
    )); // RFC 1918 172.31
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(192, 168, 1, 1).into()
    )); // RFC 1918 Home/LAN
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(169, 254, 169, 254).into()
    )); // Cloud Metadata / Link Local
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(0, 0, 0, 0).into()
    )); // Unspecified
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(224, 0, 0, 1).into()
    )); // Multicast
    assert!(is_private_or_restricted_ip(
        Ipv4Addr::new(255, 255, 255, 255).into()
    )); // Broadcast

    // IPv4 Valid Public IPs
    assert!(!is_private_or_restricted_ip(
        Ipv4Addr::new(8, 8, 8, 8).into()
    )); // Google DNS
    assert!(!is_private_or_restricted_ip(
        Ipv4Addr::new(1, 1, 1, 1).into()
    )); // Cloudflare DNS
    assert!(!is_private_or_restricted_ip(
        Ipv4Addr::new(151, 101, 1, 140).into()
    )); // Fastly

    // IPv6 Restricted Matrix
    assert!(is_private_or_restricted_ip(Ipv6Addr::LOCALHOST.into())); // ::1
    assert!(is_private_or_restricted_ip(Ipv6Addr::UNSPECIFIED.into())); // ::
    assert!(is_private_or_restricted_ip(
        "fe80::1".parse::<Ipv6Addr>().unwrap().into()
    )); // Link-local
    assert!(is_private_or_restricted_ip(
        "fc00::1".parse::<Ipv6Addr>().unwrap().into()
    )); // ULA Private

    // Hostnames
    assert!(is_restricted_hostname("localhost"));
    assert!(is_restricted_hostname("metadata.google.internal"));
    assert!(is_restricted_hostname("instance-data.ec2.internal"));
    assert!(is_restricted_hostname("myserver.local"));
    assert!(is_restricted_hostname("router.lan"));
    assert!(!is_restricted_hostname("example.com"));
    assert!(!is_restricted_hostname("maps.google.com"));

    // URL validation
    assert!(validate_url_for_ssrf("http://127.0.0.1/secret").is_err());
    assert!(validate_url_for_ssrf("http://169.254.169.254/latest/meta-data/").is_err());
    assert!(validate_url_for_ssrf("http://localhost:8080/admin").is_err());
    assert!(validate_url_for_ssrf("http://cluster.internal/").is_err());
    assert!(validate_url_for_ssrf("ftp://example.com/file").is_err()); // Non-http/https
    assert!(validate_url_for_ssrf("https://httpbin.org/get").is_ok());
}

#[tokio::test]
async fn test_fly_server_builder_and_health_endpoints() {
    let server = FlyServer::builder()
        .with_app_info("test-app", "1.2.3")
        .with_port(4000)
        .nest(
            "/api",
            Router::new().route("/ping", get(|| async { "pong" })),
        );

    let router = server.build_router();

    // 1. /health
    let res = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "ok");

    // 2. /up (Fly machine healthcheck)
    let res_up = router
        .clone()
        .oneshot(Request::builder().uri("/up").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res_up.status(), StatusCode::OK);

    // 3. /api/info
    let res_info = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_info.status(), StatusCode::OK);
    let bytes_info = res_info.into_body().collect().await.unwrap().to_bytes();
    let json_info: Value = serde_json::from_slice(&bytes_info).unwrap();
    assert_eq!(json_info["name"], "test-app");
    assert_eq!(json_info["version"], "1.2.3");

    // 4. Nested API route and security headers
    let res_ping = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ping.status(), StatusCode::OK);
    let sec_header = res_ping.headers().get("x-content-type-options").unwrap();
    assert_eq!(sec_header, "nosniff");
    let frame_header = res_ping.headers().get("x-frame-options").unwrap();
    assert_eq!(frame_header, "SAMEORIGIN");
    let bytes_ping = res_ping.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&bytes_ping[..], b"pong");

    // 5. Embedded assets
    let res_css = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/_fly/fly-base.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);
    assert_eq!(
        res_css.headers().get("content-type").unwrap(),
        "text/css; charset=utf-8"
    );
}

#[tokio::test]
async fn test_cors_preflight_and_headers() {
    let server = FlyServer::builder().nest(
        "/api",
        Router::new().route("/data", get(|| async { "data" })),
    );
    let router = server.build_router();

    let preflight_req = Request::builder()
        .method("OPTIONS")
        .uri("/api/data")
        .header("origin", "https://example.com")
        .header("access-control-request-method", "GET")
        .header("access-control-request-headers", "x-user-token")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(preflight_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_sqlite_concurrency_across_tasks() {
    let pool = FlyDb::open_shared_in_memory().expect("open shared memory");
    {
        let mut conn = pool.lock().unwrap();
        FlyDb::run_migrations(
            &mut conn,
            &[
                "CREATE TABLE counters (id INTEGER PRIMARY KEY, count INTEGER NOT NULL);",
                "INSERT INTO counters (id, count) VALUES (1, 0);",
            ],
        )
        .expect("migration");
    }

    let mut handles = vec![];
    for _ in 0..10 {
        let pool_clone = pool.clone();
        handles.push(tokio::spawn(async move {
            let conn = pool_clone.lock().unwrap();
            conn.execute("UPDATE counters SET count = count + 1 WHERE id = 1", [])
                .expect("update");
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let total: i64 = pool
        .lock()
        .unwrap()
        .query_row("SELECT count FROM counters WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("query total");
    assert_eq!(total, 10);
}

#[test]
fn test_safe_http_client_builder() {
    let client = build_safe_http_client(std::time::Duration::from_secs(5));
    // Verify client was constructed
    let _ = client;
}

#[test]
fn test_pure_rust_qr_svg_generator() {
    let qr = generate_qr_svg("https://blist-radmuffin.fly.dev/?sync=usr_test", 240, 2);
    assert!(qr.svg.contains("<svg"));
    assert!(qr.svg.contains("viewBox="));
    assert!(qr.data_url.starts_with("data:image/svg+xml;charset=utf-8,"));
    assert_eq!(qr.size, 240);
}

#[cfg(feature = "sync")]
#[test]
fn test_sync_token_generation() {
    let share_tok = generate_share_token();
    assert_eq!(share_tok.len(), 32);

    let user_tok = generate_user_token("usr_");
    assert!(user_tok.starts_with("usr_"));
    assert_eq!(user_tok.len(), 36);
}

#[cfg(feature = "ws")]
#[tokio::test]
async fn test_ws_hub_integration() {
    let hub = BroadcastHub::new(32);
    let mut sub = hub.subscribe("room_alpha");

    let msg = WsMessage {
        room: "room_alpha".to_string(),
        event: "doc_updated".to_string(),
        sender_token: Some("usr_creator".to_string()),
        payload: serde_json::json!({ "version": 2, "delta": "added paragraph" }),
    };

    let delivered = hub.broadcast(msg).expect("broadcast");
    assert_eq!(delivered, 1);

    let received = sub.recv().await.expect("recv");
    assert_eq!(received.event, "doc_updated");
    assert_eq!(received.payload["version"], 2);
}

#[cfg(feature = "io")]
#[test]
fn test_csv_and_chunking_integration() {
    #[derive(Debug, serde::Deserialize, PartialEq, Eq)]
    struct TaskItem {
        title: String,
        status: String,
    }

    let csv_content = "title,status\nBuild Fly App,In Progress\nDeploy to Production,Done\n";
    let tasks: Vec<TaskItem> = parse_csv(csv_content.as_bytes()).expect("parse csv");
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].title, "Build Fly App");
    assert_eq!(tasks[1].status, "Done");

    let chunked: Vec<_> = chunk_slice(&tasks, 1).collect();
    assert_eq!(chunked.len(), 2);
}
