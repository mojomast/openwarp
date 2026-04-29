use serde_json::json;
use tokio::sync::{mpsc, watch};

use crate::api::ApiError;
use crate::pty::{PtyClient, PtyEvent, PtyState};

enum SessionControl {
    Resize { rows: usize, cols: usize },
}

pub struct PtySession {
    pub input_tx: mpsc::Sender<String>,
    pub state_rx: watch::Receiver<PtyState>,
    control_tx: mpsc::Sender<SessionControl>,
    handle: tokio::task::JoinHandle<()>,
}

impl PtySession {
    pub async fn connect(
        client: PtyClient,
        pty_id: String,
        initial_rows: usize,
        initial_cols: usize,
    ) -> Result<Self, ApiError> {
        let (input_tx, mut event_rx) = client.connect(&pty_id, None).await?;
        let state = PtyState::new(initial_rows, initial_cols);
        let (state_tx, state_rx) = watch::channel(state.clone());
        let (control_tx, mut control_rx) = mpsc::channel(16);

        let handle = tokio::spawn(async move {
            let mut state = state;
            loop {
                tokio::select! {
                    Some(control) = control_rx.recv() => {
                        match control {
                            SessionControl::Resize { rows, cols } => {
                                state.resize(rows, cols);
                                let _ = state_tx.send(state.clone());
                            }
                        }
                    }
                    event = event_rx.recv() => {
                        match event {
                            Some(PtyEvent::Output(text)) => {
                                state.feed(text.as_bytes());
                                let _ = state_tx.send(state.clone());
                            }
                            Some(PtyEvent::Cursor(_)) => {}
                            Some(PtyEvent::Closed) | None => {
                                state.feed("\r\n\x1b[1;31m[Connection closed]\x1b[0m");
                                let _ = state_tx.send(state.clone());
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            input_tx,
            state_rx,
            control_tx,
            handle,
        })
    }

    pub async fn resize(&self, rows: usize, cols: usize) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let _ = self
            .control_tx
            .send(SessionControl::Resize { rows, cols })
            .await;
        let message = json!({
            "type": "resize",
            "cols": cols,
            "rows": rows,
        })
        .to_string();
        let _ = self.input_tx.send(message).await;
    }

    pub fn abort(&self) {
        self.handle.abort();
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
