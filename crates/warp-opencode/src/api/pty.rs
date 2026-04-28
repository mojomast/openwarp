use super::client::{ApiClient, ApiError};
use super::schema::{PtyCreateRequest, PtyInfo, PtyUpdateRequest};

impl ApiClient {
    pub async fn list_ptys(&self) -> Result<Vec<PtyInfo>, ApiError> {
        self.get("/pty/").await
    }

    pub async fn create_pty(&self, input: &PtyCreateRequest) -> Result<PtyInfo, ApiError> {
        self.post("/pty/", input).await
    }

    pub async fn update_pty(
        &self,
        pty_id: &str,
        input: &PtyUpdateRequest,
    ) -> Result<Option<PtyInfo>, ApiError> {
        self.put(&format!("/pty/{pty_id}"), input).await
    }

    pub async fn delete_pty(&self, pty_id: &str) -> Result<bool, ApiError> {
        self.delete(&format!("/pty/{pty_id}")).await
    }
}
