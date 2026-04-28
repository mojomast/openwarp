use super::client::{ApiClient, ApiError};
use super::schema::{QuestionReply, QuestionRequest};

impl ApiClient {
    pub async fn list_questions(&self) -> Result<Vec<QuestionRequest>, ApiError> {
        self.get("/question/").await
    }

    pub async fn reply_question(
        &self,
        request_id: &str,
        reply: &QuestionReply,
    ) -> Result<bool, ApiError> {
        self.post(&format!("/question/{request_id}/reply"), reply)
            .await
    }

    pub async fn reject_question(&self, request_id: &str) -> Result<bool, ApiError> {
        self.post_empty(&format!("/question/{request_id}/reject"))
            .await
    }
}
