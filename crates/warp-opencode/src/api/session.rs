use super::client::{ApiClient, ApiError};
use super::schema::*;
use std::collections::HashMap;

impl ApiClient {
    pub async fn health(&self) -> Result<(), ApiError> {
        let response = self.http().get(self.url("/health")?).send().await?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ApiError::Status { status, body })
        }
    }

    pub async fn list_sessions(&self) -> Result<Vec<Session>, ApiError> {
        self.get_or_default("/session").await
    }

    pub async fn session_status(&self) -> Result<HashMap<SessionId, SessionStatus>, ApiError> {
        self.get("/session/status").await
    }

    pub async fn create_session(&self, input: Option<&SessionCreate>) -> Result<Session, ApiError> {
        match input {
            Some(input) => self.post("/session", input).await,
            None => self.post("/session", &serde_json::Value::Null).await,
        }
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<bool, ApiError> {
        self.delete(&format!("/session/{session_id}")).await
    }

    pub async fn list_messages(&self, session_id: &str) -> Result<Vec<MessageWithParts>, ApiError> {
        self.get(&format!("/session/{session_id}/message")).await
    }

    pub async fn send_message(
        &self,
        session_id: &str,
        request: &SendMessageRequest,
    ) -> Result<MessageWithParts, ApiError> {
        self.post(&format!("/session/{session_id}/message"), request)
            .await
    }

    pub async fn prompt_async(
        &self,
        session_id: &str,
        request: &SendMessageRequest,
    ) -> Result<(), ApiError> {
        let response = self
            .http()
            .post(self.url(&format!("/session/{session_id}/prompt_async"))?)
            .json(request)
            .send()
            .await?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(ApiError::Status {
                status,
                body: response.text().await?,
            })
        }
    }

    pub async fn abort_session(&self, session_id: &str) -> Result<bool, ApiError> {
        self.post_empty(&format!("/session/{session_id}/abort"))
            .await
    }
}
