use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub type SessionId = String;
pub type MessageId = String;
pub type PartId = String;
pub type PermissionId = String;
pub type QuestionId = String;
pub type PtyId = String;
pub type ProviderId = String;
pub type ModelId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub healthy: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: SessionId,
    pub slug: Option<String>,
    pub title: String,
    pub directory: Option<String>,
    #[serde(default)]
    pub time: Value,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreate {
    #[serde(rename = "parentID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<SessionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Vec<PermissionRule>>,
    #[serde(rename = "workspaceID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    pub permission: String,
    pub pattern: String,
    pub action: PermissionAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: ProviderId,
    #[serde(rename = "modelID")]
    pub model_id: ModelId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    #[serde(rename = "messageID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_reply: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashMap<String, bool>>,
    pub parts: Vec<PromptPartInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PromptPartInput {
    Text {
        text: String,
    },
    File {
        mime: String,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageWithParts {
    pub info: MessageInfo,
    #[serde(default)]
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageInfo {
    pub id: MessageId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    pub role: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub id: PartId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub state: Option<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SessionStatus {
    Idle,
    Busy,
    Retry {
        attempt: u32,
        message: String,
        next: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub id: PermissionId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    pub permission: String,
    pub patterns: Vec<String>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub always: Vec<String>,
    #[serde(default)]
    pub tool: Option<ToolRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolRef {
    #[serde(rename = "messageID")]
    pub message_id: MessageId,
    pub call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionReplyKind {
    Once,
    Always,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PermissionReply {
    pub reply: PermissionReplyKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRequest {
    pub id: QuestionId,
    #[serde(rename = "sessionID")]
    pub session_id: SessionId,
    pub questions: Vec<Question>,
    #[serde(default)]
    pub tool: Option<ToolRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    #[serde(default)]
    pub multiple: Option<bool>,
    #[serde(default)]
    pub custom: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionReply {
    pub answers: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderListResult {
    pub all: Vec<ProviderInfo>,
    #[serde(default)]
    pub default: HashMap<String, String>,
    #[serde(default)]
    pub connected: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub name: String,
    #[serde(default)]
    pub models: HashMap<String, Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PtyCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PtyInfo {
    pub id: PtyId,
    pub title: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: String,
    pub status: String,
    #[serde(default)]
    pub pid: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PtyUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<PtySize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}
