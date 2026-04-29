use super::client::{ApiClient, ApiError};
use super::schema::{PermissionReply, PermissionRequest};

impl ApiClient {
    pub async fn list_permissions(&self) -> Result<Vec<PermissionRequest>, ApiError> {
        self.get_or_default("/permission").await
    }

    pub async fn reply_permission(
        &self,
        request_id: &str,
        reply: &PermissionReply,
    ) -> Result<bool, ApiError> {
        self.post(&format!("/permission/{request_id}/reply"), reply)
            .await
    }
}
