use crate::api::schema::{PermissionReply, PermissionReplyKind};
use crate::api::{ApiClient, ApiConfig, Auth, EventStream};
use crate::state::{AppStore, ConnectionStatus};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::task::JoinHandle;

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const AUTH_USERNAME: &str = "opencode";

/// Background task that keeps the OpenCode server-sent event stream connected
/// and mirrors incoming events into the shared application store.
pub struct SseLoop {
    store: AppStore,
    base_url: String,
    token: String,
}

impl SseLoop {
    pub fn new(store: AppStore, base_url: String, token: String) -> Self {
        Self {
            store,
            base_url,
            token,
        }
    }

    /// Spawn the reconnecting SSE loop on the current Tokio runtime.
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }

    async fn run(self) {
        let client = match self.client() {
            Ok(client) => client,
            Err(err) => {
                self.store
                    .set_connection(ConnectionStatus::Error(err.to_string()))
                    .await;
                return;
            }
        };

        let mut attempt = 0_u32;
        let mut backoff = INITIAL_BACKOFF;

        loop {
            if attempt > 0 {
                self.store
                    .set_connection(ConnectionStatus::Reconnecting { attempt })
                    .await;
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }

            match EventStream::connect(client.clone()).await {
                Ok(mut stream) => {
                    self.store.set_connection(ConnectionStatus::Connected).await;
                    attempt = 0;
                    backoff = INITIAL_BACKOFF;

                    let mut disconnect_reason = "event stream closed".to_string();
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(event) => {
                                self.store.apply_event(event).await;
                                for request_id in self.store.drain_auto_approvals().await {
                                    let reply = PermissionReply {
                                        reply: PermissionReplyKind::Always,
                                        message: None,
                                    };
                                    if let Err(error) =
                                        client.reply_permission(&request_id, &reply).await
                                    {
                                        tracing::warn!(%request_id, %error, "failed to auto-approve permission");
                                    }
                                }
                            }
                            Err(err) => {
                                disconnect_reason = err.to_string();
                                break;
                            }
                        }
                    }

                    self.store
                        .set_connection(ConnectionStatus::Error(disconnect_reason))
                        .await;
                }
                Err(err) => {
                    self.store
                        .set_connection(ConnectionStatus::Error(err.to_string()))
                        .await;
                }
            }

            attempt = attempt.saturating_add(1);
        }
    }

    fn client(&self) -> Result<ApiClient, crate::api::ApiError> {
        let mut config = ApiConfig::new(&self.base_url)?;
        if !self.token.is_empty() {
            config.auth = Auth::Basic {
                username: AUTH_USERNAME.to_string(),
                password: self.token.clone(),
            };
        }
        ApiClient::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{header, method, path};
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
    async fn applies_events_from_sse_stream() {
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"type\":\"session.created\",\"properties\":{",
            "\"sessionID\":\"session-1\",",
            "\"info\":{\"id\":\"session-1\",\"slug\":null,\"title\":\"Test\",\"directory\":null}",
            "}}\n\n"
        );

        Mock::given(method("GET"))
            .and(path("/event"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;

        let store = AppStore::default();
        let handle = SseLoop::new(store.clone(), server.uri(), String::new()).spawn();

        wait_until(|| {
            let store = store.clone();
            async move {
                store
                    .snapshot()
                    .await
                    .sessions
                    .iter()
                    .any(|session| session.id == "session-1")
            }
        })
        .await;

        handle.abort();
    }

    #[tokio::test]
    async fn sends_basic_auth_when_token_is_configured() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/event"))
            .and(header("authorization", "Basic b3BlbmNvZGU6c2VjcmV0"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let store = AppStore::default();
        let handle = SseLoop::new(store.clone(), server.uri(), "secret".to_string()).spawn();

        wait_until(|| {
            let store = store.clone();
            async move {
                matches!(
                    store.snapshot().await.connection,
                    ConnectionStatus::Error(_) | ConnectionStatus::Reconnecting { .. }
                )
            }
        })
        .await;

        handle.abort();
    }
}
