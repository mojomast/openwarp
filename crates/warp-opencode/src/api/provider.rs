use super::client::{ApiClient, ApiError};
use super::schema::ProviderListResult;

impl ApiClient {
    pub async fn list_providers(&self) -> Result<ProviderListResult, ApiError> {
        self.get("/provider/").await
    }
}
