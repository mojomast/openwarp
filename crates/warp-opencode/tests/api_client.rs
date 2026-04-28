use serde_json::json;
use warp_opencode::api::events::{event_from_json, OpenCodeEvent};
use warp_opencode::api::schema::*;
use warp_opencode::api::{ApiClient, ApiConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn creates_session_and_sends_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/session/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id":"ses_1","title":"Test","time":{}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/session/ses_1/message"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"info":{"id":"msg_2","sessionID":"ses_1","role":"assistant"},"parts":[]}),
        ))
        .mount(&server)
        .await;

    let client = ApiClient::new(ApiConfig::new(server.uri()).unwrap()).unwrap();
    let session = client
        .create_session(Some(&SessionCreate {
            parent_id: None,
            title: Some("Test".to_string()),
            permission: None,
            workspace_id: None,
        }))
        .await
        .unwrap();
    assert_eq!(session.id, "ses_1");

    let response = client
        .send_message(
            &session.id,
            &SendMessageRequest {
                message_id: None,
                model: None,
                agent: None,
                no_reply: None,
                tools: None,
                parts: vec![PromptPartInput::Text {
                    text: "hello".to_string(),
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(response.info.id, "msg_2");
}

#[test]
fn decodes_permission_event() {
    let event = event_from_json(json!({
        "type":"permission.asked",
        "properties":{"id":"perm_1","sessionID":"ses_1","permission":"bash","patterns":["*"],"metadata":{},"always":[]}
    })).unwrap();
    assert!(matches!(event, OpenCodeEvent::PermissionAsked(request) if request.id == "perm_1"));
}

#[test]
fn applies_part_delta_incrementally() {
    let mut model = warp_opencode::state::AppModel::default();
    model.apply_event(OpenCodeEvent::MessagePartUpdated {
        session_id: "ses_1".to_string(),
        time: None,
        part: Part {
            id: "part_1".to_string(),
            session_id: "ses_1".to_string(),
            message_id: "msg_1".to_string(),
            kind: "text".to_string(),
            text: Some("hel".to_string()),
            state: None,
            extra: Default::default(),
        },
    });
    model.apply_event(OpenCodeEvent::MessagePartDelta {
        session_id: "ses_1".to_string(),
        message_id: "msg_1".to_string(),
        part_id: "part_1".to_string(),
        field: "text".to_string(),
        delta: "lo".to_string(),
    });
    let text = model.threads["ses_1"].messages[0].parts[0].text.as_deref();
    assert_eq!(text, Some("hello"));
}
