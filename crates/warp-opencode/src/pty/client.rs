use crate::api::{ApiClient, ApiError};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, PartialEq)]
pub enum PtyEvent {
    Output(String),
    Cursor(u64),
    Closed,
}

#[derive(Clone)]
pub struct PtyClient {
    api: ApiClient,
}

impl PtyClient {
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }

    pub async fn connect(
        &self,
        pty_id: &str,
        cursor: Option<i64>,
    ) -> Result<(mpsc::Sender<String>, mpsc::Receiver<PtyEvent>), ApiError> {
        let cursor = cursor.unwrap_or(-1);
        let url = self
            .api
            .websocket_url(&format!("/pty/{pty_id}/connect?cursor={cursor}"))?;
        let (socket, _) = connect_async(url.as_str())
            .await
            .map_err(|err| ApiError::InvalidHeader(err.to_string()))?;
        let (mut sink, mut stream) = socket.split();
        let (input_tx, mut input_rx) = mpsc::channel::<String>(128);
        let (event_tx, event_rx) = mpsc::channel::<PtyEvent>(128);

        tokio::spawn(async move {
            while let Some(input) = input_rx.recv().await {
                if sink.send(Message::Text(input.into())).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            while let Some(frame) = stream.next().await {
                match frame {
                    Ok(Message::Text(text)) => {
                        let _ = event_tx.send(PtyEvent::Output(text.to_string())).await;
                    }
                    Ok(Message::Binary(bytes)) => {
                        if bytes.first() == Some(&0) {
                            if let Ok(cursor) = serde_json::from_slice::<CursorFrame>(&bytes[1..]) {
                                let _ = event_tx.send(PtyEvent::Cursor(cursor.cursor)).await;
                            }
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
            let _ = event_tx.send(PtyEvent::Closed).await;
        });

        Ok((input_tx, event_rx))
    }
}

#[derive(Deserialize)]
struct CursorFrame {
    cursor: u64,
}
