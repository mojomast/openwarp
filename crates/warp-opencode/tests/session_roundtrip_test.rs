use std::time::Duration;

use serde_json::json;
use warp_opencode::api::{ApiClient, ApiConfig};
use warp_opencode::sse_loop::SseLoop;
use warp_opencode::state::AppStore;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn wait_until<F, Fut>(mut predicate: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if predicate().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("condition was not met before timeout");
}

#[tokio::test]
async fn bootstraps_sessions_and_applies_sse_event_to_store() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/session/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id":"ses_1","title":"Existing","time":{}},
            {"id":"ses_2","title":"Second","time":{}}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/session/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ses_1":{"type":"idle"}})))
        .mount(&server)
        .await;

    let sse_body = concat!(
        "data: {\"type\":\"message.part.updated\",\"properties\":{",
        "\"sessionID\":\"ses_1\",",
        "\"part\":{\"id\":\"part_1\",\"sessionID\":\"ses_1\",\"messageID\":\"msg_1\",\"type\":\"text\",\"text\":\"hello from sse\"}",
        "}}\n\n"
    );
    Mock::given(method("GET"))
        .and(path("/event"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let client = ApiClient::new(ApiConfig::new(server.uri()).unwrap()).unwrap();
    let store = AppStore::default();

    let sessions = client.list_sessions().await.unwrap();
    let statuses = client.session_status().await.unwrap();
    store
        .replace_bootstrap(sessions, statuses, Vec::new(), Vec::new(), None)
        .await;

    let handle = SseLoop::new(store.clone(), server.uri(), String::new()).spawn();
    wait_until(|| {
        let store = store.clone();
        async move {
            store
                .snapshot()
                .await
                .threads
                .get("ses_1")
                .and_then(|thread| thread.messages.first())
                .and_then(|message| message.parts.first())
                .and_then(|part| part.text.as_deref())
                == Some("hello from sse")
        }
    })
    .await;
    handle.abort();

    let model = store.snapshot().await;
    assert_eq!(model.sessions.len(), 2);
    assert_eq!(model.active_session_id.as_deref(), Some("ses_1"));
    assert!(matches!(
        model.statuses.get("ses_1"),
        Some(warp_opencode::api::schema::SessionStatus::Idle)
    ));
    assert_eq!(
        model.threads["ses_1"].messages[0].parts[0].text.as_deref(),
        Some("hello from sse")
    );
}
