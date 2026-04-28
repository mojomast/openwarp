use super::client::{ApiClient, ApiError};
use super::schema::*;
use futures_util::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq)]
pub enum OpenCodeEvent {
    SessionCreated {
        session_id: SessionId,
        info: Session,
    },
    SessionUpdated {
        session_id: SessionId,
        info: Session,
    },
    SessionDeleted {
        session_id: SessionId,
        info: Session,
    },
    MessageUpdated {
        session_id: SessionId,
        info: MessageInfo,
    },
    MessageRemoved {
        session_id: SessionId,
        message_id: MessageId,
    },
    MessagePartUpdated {
        session_id: SessionId,
        part: Part,
        time: Option<u64>,
    },
    MessagePartRemoved {
        session_id: SessionId,
        message_id: MessageId,
        part_id: PartId,
    },
    MessagePartDelta {
        session_id: SessionId,
        message_id: MessageId,
        part_id: PartId,
        field: String,
        delta: String,
    },
    SessionStatus {
        session_id: SessionId,
        status: SessionStatus,
    },
    SessionIdle {
        session_id: SessionId,
    },
    PermissionAsked(PermissionRequest),
    PermissionReplied {
        session_id: SessionId,
        request_id: PermissionId,
        reply: String,
    },
    QuestionAsked(QuestionRequest),
    QuestionReplied {
        session_id: SessionId,
        request_id: QuestionId,
        answers: Vec<Vec<String>>,
    },
    QuestionRejected {
        session_id: SessionId,
        request_id: QuestionId,
    },
    PtyCreated {
        info: PtyInfo,
    },
    PtyUpdated {
        info: PtyInfo,
    },
    PtyExited {
        id: PtyId,
        exit_code: Option<i64>,
    },
    PtyDeleted {
        id: PtyId,
    },
    Unknown {
        kind: String,
        properties: Value,
    },
}

#[derive(Debug, Deserialize)]
struct RawBusEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    properties: Value,
}

pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = Result<OpenCodeEvent, ApiError>> + Send>>,
}

impl EventStream {
    pub async fn connect(client: ApiClient) -> Result<Self, ApiError> {
        let response = client.http().get(client.url("/event")?).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Status {
                status,
                body: response.text().await?,
            });
        }
        let stream = response.bytes_stream().map(|chunk| match chunk {
            Ok(bytes) => parse_sse_chunk(&bytes),
            Err(err) => Err(ApiError::Reqwest(err)),
        });
        Ok(Self {
            inner: Box::pin(stream),
        })
    }
}

impl Stream for EventStream {
    type Item = Result<OpenCodeEvent, ApiError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

fn parse_sse_chunk(bytes: &[u8]) -> Result<OpenCodeEvent, ApiError> {
    let text = String::from_utf8_lossy(bytes);
    let data = text
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    let data = if data.is_empty() {
        text.trim()
    } else {
        data.trim()
    };
    let raw: RawBusEvent = serde_json::from_str(data)?;
    event_from_raw(raw)
}

pub fn event_from_json(value: Value) -> Result<OpenCodeEvent, ApiError> {
    event_from_raw(serde_json::from_value(value)?)
}

fn event_from_raw(raw: RawBusEvent) -> Result<OpenCodeEvent, ApiError> {
    let p = raw.properties;
    Ok(match raw.kind.as_str() {
        "session.created" => OpenCodeEvent::SessionCreated {
            session_id: get(&p, "sessionID")?,
            info: serde_json::from_value(p["info"].clone())?,
        },
        "session.updated" => OpenCodeEvent::SessionUpdated {
            session_id: get(&p, "sessionID")?,
            info: serde_json::from_value(p["info"].clone())?,
        },
        "session.deleted" => OpenCodeEvent::SessionDeleted {
            session_id: get(&p, "sessionID")?,
            info: serde_json::from_value(p["info"].clone())?,
        },
        "message.updated" => OpenCodeEvent::MessageUpdated {
            session_id: get(&p, "sessionID")?,
            info: serde_json::from_value(p["info"].clone())?,
        },
        "message.removed" => OpenCodeEvent::MessageRemoved {
            session_id: get(&p, "sessionID")?,
            message_id: get(&p, "messageID")?,
        },
        "message.part.updated" => OpenCodeEvent::MessagePartUpdated {
            session_id: get(&p, "sessionID")?,
            part: serde_json::from_value(p["part"].clone())?,
            time: p.get("time").and_then(Value::as_u64),
        },
        "message.part.removed" => OpenCodeEvent::MessagePartRemoved {
            session_id: get(&p, "sessionID")?,
            message_id: get(&p, "messageID")?,
            part_id: get(&p, "partID")?,
        },
        "message.part.delta" => OpenCodeEvent::MessagePartDelta {
            session_id: get(&p, "sessionID")?,
            message_id: get(&p, "messageID")?,
            part_id: get(&p, "partID")?,
            field: get(&p, "field")?,
            delta: get(&p, "delta")?,
        },
        "session.status" => OpenCodeEvent::SessionStatus {
            session_id: get(&p, "sessionID")?,
            status: serde_json::from_value(p["status"].clone())?,
        },
        "session.idle" => OpenCodeEvent::SessionIdle {
            session_id: get(&p, "sessionID")?,
        },
        "permission.asked" => OpenCodeEvent::PermissionAsked(serde_json::from_value(p)?),
        "permission.replied" => OpenCodeEvent::PermissionReplied {
            session_id: get(&p, "sessionID")?,
            request_id: get(&p, "requestID")?,
            reply: get(&p, "reply")?,
        },
        "question.asked" => OpenCodeEvent::QuestionAsked(serde_json::from_value(p)?),
        "question.replied" => OpenCodeEvent::QuestionReplied {
            session_id: get(&p, "sessionID")?,
            request_id: get(&p, "requestID")?,
            answers: serde_json::from_value(p["answers"].clone())?,
        },
        "question.rejected" => OpenCodeEvent::QuestionRejected {
            session_id: get(&p, "sessionID")?,
            request_id: get(&p, "requestID")?,
        },
        "pty.created" => OpenCodeEvent::PtyCreated {
            info: serde_json::from_value(p["info"].clone())?,
        },
        "pty.updated" => OpenCodeEvent::PtyUpdated {
            info: serde_json::from_value(p["info"].clone())?,
        },
        "pty.exited" => OpenCodeEvent::PtyExited {
            id: get(&p, "id")?,
            exit_code: p.get("exitCode").and_then(Value::as_i64),
        },
        "pty.deleted" => OpenCodeEvent::PtyDeleted { id: get(&p, "id")? },
        other => OpenCodeEvent::Unknown {
            kind: other.to_string(),
            properties: p,
        },
    })
}

fn get(value: &Value, key: &str) -> Result<String, ApiError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("missing string field {key}"),
            ))
            .into()
        })
}
