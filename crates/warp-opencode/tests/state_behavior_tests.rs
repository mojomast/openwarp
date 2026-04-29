use std::collections::HashMap;

use serde_json::json;
use warp_opencode::api::events::OpenCodeEvent;
use warp_opencode::api::schema::*;
use warp_opencode::state::{AppModel, AppStore, ConnectionStatus, SessionThread};

fn session(id: &str, title: &str) -> Session {
    Session {
        id: id.to_string(),
        slug: None,
        title: title.to_string(),
        directory: None,
        time: json!({}),
        extra: HashMap::new(),
    }
}

fn message_info(session_id: &str, message_id: &str, role: &str) -> MessageInfo {
    MessageInfo {
        id: message_id.to_string(),
        session_id: session_id.to_string(),
        role: role.to_string(),
        extra: HashMap::new(),
    }
}

fn text_part(session_id: &str, message_id: &str, part_id: &str, text: impl Into<String>) -> Part {
    Part {
        id: part_id.to_string(),
        session_id: session_id.to_string(),
        message_id: message_id.to_string(),
        kind: "text".to_string(),
        text: Some(text.into()),
        state: None,
        extra: HashMap::new(),
    }
}

fn permission(id: &str, session_id: &str) -> PermissionRequest {
    PermissionRequest {
        id: id.to_string(),
        session_id: session_id.to_string(),
        permission: "bash".to_string(),
        patterns: vec!["*".to_string()],
        metadata: json!({}),
        always: Vec::new(),
        tool: None,
    }
}

fn permission_with_tool(id: &str, session_id: &str, tool_name: &str) -> PermissionRequest {
    PermissionRequest {
        id: id.to_string(),
        session_id: session_id.to_string(),
        permission: tool_name.to_string(),
        patterns: vec!["*".to_string()],
        metadata: json!({}),
        always: Vec::new(),
        tool: None,
    }
}

#[test]
fn session_activation_survives_upserts_and_moves_on_delete() {
    let mut model = AppModel::default();

    model.upsert_session(session("ses_1", "First"));
    model.upsert_session(session("ses_2", "Second"));
    assert_eq!(model.active_session_id.as_deref(), Some("ses_1"));

    model.apply_event(OpenCodeEvent::SessionUpdated {
        session_id: "ses_1".to_string(),
        info: session("ses_1", "Renamed"),
    });
    assert_eq!(model.active_session_id.as_deref(), Some("ses_1"));
    assert_eq!(
        model
            .sessions
            .iter()
            .find(|s| s.id == "ses_1")
            .unwrap()
            .title,
        "Renamed"
    );

    model.apply_event(OpenCodeEvent::MessagePartUpdated {
        session_id: "ses_1".to_string(),
        part: text_part("ses_1", "msg_1", "part_1", "cached"),
        time: None,
    });
    assert!(model.threads.contains_key("ses_1"));

    model.apply_event(OpenCodeEvent::SessionDeleted {
        session_id: "ses_1".to_string(),
        info: session("ses_1", "Renamed"),
    });

    assert_eq!(model.active_session_id.as_deref(), Some("ses_2"));
    assert!(!model.sessions.iter().any(|s| s.id == "ses_1"));
    assert!(!model.threads.contains_key("ses_1"));
}

#[tokio::test]
async fn store_remove_active_session_clears_status_and_selects_next() {
    let store = AppStore::new(AppModel::default());
    store.upsert_session(session("ses_1", "First")).await;
    store.upsert_session(session("ses_2", "Second")).await;
    store
        .apply_event(OpenCodeEvent::SessionStatus {
            session_id: "ses_1".to_string(),
            status: SessionStatus::Busy,
        })
        .await;

    store.remove_session("ses_1").await;
    let model = store.snapshot().await;
    assert_eq!(model.active_session_id.as_deref(), Some("ses_2"));
    assert!(!model.statuses.contains_key("ses_1"));
}

#[test]
fn permission_queue_adds_and_removes_by_request_id() {
    let mut model = AppModel::default();

    model.apply_event(OpenCodeEvent::PermissionAsked(permission(
        "perm_1", "ses_1",
    )));
    model.apply_event(OpenCodeEvent::PermissionAsked(permission(
        "perm_2", "ses_1",
    )));
    assert_eq!(model.permissions.len(), 2);

    model.apply_event(OpenCodeEvent::PermissionReplied {
        session_id: "ses_1".to_string(),
        request_id: "perm_1".to_string(),
        reply: "once".to_string(),
    });
    assert!(!model.permissions.contains_key("perm_1"));
    assert!(model.permissions.contains_key("perm_2"));
}

#[test]
fn message_delta_accumulates_text_and_ignores_unknown_fields() {
    let mut model = AppModel::default();
    model.apply_event(OpenCodeEvent::MessagePartUpdated {
        session_id: "ses_1".to_string(),
        part: text_part("ses_1", "msg_1", "part_1", "hel"),
        time: None,
    });
    model.apply_event(OpenCodeEvent::MessagePartDelta {
        session_id: "ses_1".to_string(),
        message_id: "msg_1".to_string(),
        part_id: "part_1".to_string(),
        field: "text".to_string(),
        delta: "lo".to_string(),
    });
    model.apply_event(OpenCodeEvent::MessagePartDelta {
        session_id: "ses_1".to_string(),
        message_id: "msg_1".to_string(),
        part_id: "part_1".to_string(),
        field: "state".to_string(),
        delta: "ignored".to_string(),
    });

    let text = model.threads["ses_1"].messages[0].parts[0].text.as_deref();
    assert_eq!(text, Some("hello"));
}

#[test]
fn replace_messages_rebuilds_part_index_for_later_updates() {
    let mut thread = SessionThread::default();
    thread.replace_messages(vec![MessageWithParts {
        info: message_info("ses_1", "msg_1", "assistant"),
        parts: vec![text_part("ses_1", "msg_1", "part_1", "old")],
    }]);
    assert_eq!(thread.messages[0].parts[0].text.as_deref(), Some("old"));

    thread.replace_messages(vec![MessageWithParts {
        info: message_info("ses_1", "msg_2", "assistant"),
        parts: vec![text_part("ses_1", "msg_2", "part_2", "new")],
    }]);

    // Public APIs expose replace_messages, but not direct delta application on a thread.
    // Replacing twice still verifies the internal index is rebuilt rather than retaining stale parts.
    assert_eq!(thread.messages.len(), 1);
    assert_eq!(thread.messages[0].info.id, "msg_2");
    assert_eq!(thread.messages[0].parts[0].id, "part_2");
}

#[tokio::test]
async fn reconnect_status_can_be_observed_after_first_disconnect() {
    let store = AppStore::default();
    store
        .set_connection(ConnectionStatus::Error("event stream closed".to_string()))
        .await;
    store
        .set_connection(ConnectionStatus::Reconnecting { attempt: 1 })
        .await;

    assert_eq!(
        store.snapshot().await.connection,
        ConnectionStatus::Reconnecting { attempt: 1 }
    );
}

#[tokio::test]
async fn always_allowed_tool_auto_queues_matching_permission() {
    let store = AppStore::default();
    store
        .apply_event(OpenCodeEvent::PermissionAsked(permission_with_tool(
            "perm_1", "ses_1", "bash",
        )))
        .await;
    assert!(store.snapshot().await.permissions.contains_key("perm_1"));

    store
        .always_allow_tool("bash".to_string(), "ses_1".to_string())
        .await;
    store
        .apply_event(OpenCodeEvent::PermissionAsked(permission_with_tool(
            "perm_2", "ses_1", "bash",
        )))
        .await;

    let model = store.snapshot().await;
    assert!(!model.permissions.contains_key("perm_2"));
    assert_eq!(model.pending_auto_approvals, vec!["perm_2".to_string()]);

    let auto = store.drain_auto_approvals().await;
    assert_eq!(auto, vec!["perm_2".to_string()]);
    assert!(store.snapshot().await.pending_auto_approvals.is_empty());
}
