use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message;
use warp_opencode::api::{ApiClient, ApiConfig};
use warp_opencode::pty::{CellColor, PtyClient, PtySession, PtyState};

fn line(state: &PtyState, row: usize) -> String {
    (0..state.grid().cols())
        .map(|col| state.grid().cell(row, col).unwrap().ch)
        .collect::<String>()
        .trim_end()
        .to_string()
}

async fn websocket_server<F, Fut>(handler: F) -> String
where
    F: FnOnce(tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        handler(socket).await;
    });
    format!("http://{addr}")
}

fn client(base_url: &str) -> PtyClient {
    PtyClient::new(ApiClient::new(ApiConfig::new(base_url).unwrap()).unwrap())
}

async fn wait_for<F>(
    state_rx: &mut tokio::sync::watch::Receiver<PtyState>,
    predicate: F,
) -> PtyState
where
    F: Fn(&PtyState) -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = state_rx.borrow().clone();
        if predicate(&snapshot) {
            return snapshot;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "PTY state did not reach expected condition"
        );
        state_rx.changed().await.unwrap();
    }
}

#[tokio::test]
async fn output_feeds_terminal_grid() {
    let base_url = websocket_server(|mut socket| async move {
        socket
            .send(Message::Text("Hello\r\n\x1b[1;32mWorld\x1b[0m".into()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    })
    .await;

    let session = PtySession::connect(client(&base_url), "pty_1".to_string(), 4, 20)
        .await
        .unwrap();
    let mut state_rx = session.state_rx.clone();
    let state = wait_for(&mut state_rx, |state| line(state, 1) == "World").await;

    assert_eq!(line(&state, 0), "Hello");
    let world = state.grid().cell(1, 0).unwrap();
    assert_eq!(world.ch, 'W');
    assert!(world.bold);
    assert_eq!(world.fg, CellColor::Indexed(2));
}

#[tokio::test]
async fn closed_event_publishes_final_snapshot() {
    let base_url = websocket_server(|mut socket| async move {
        socket.send(Message::Text("done".into())).await.unwrap();
        socket.close(None).await.unwrap();
    })
    .await;

    let session = PtySession::connect(client(&base_url), "pty_1".to_string(), 4, 40)
        .await
        .unwrap();
    let mut state_rx = session.state_rx.clone();
    let state = wait_for(&mut state_rx, |state| {
        (0..state.grid().rows()).any(|row| line(state, row).contains("[Connection closed]"))
    })
    .await;

    assert!(line(&state, 1).contains("[Connection closed]"));
}

#[tokio::test]
async fn resize_sends_json_message() {
    let (message_tx, message_rx) = oneshot::channel();
    let base_url = websocket_server(|mut socket| async move {
        while let Some(frame) = socket.next().await {
            let frame = frame.unwrap();
            if let Message::Text(text) = frame {
                let _ = message_tx.send(text.to_string());
                break;
            }
        }
    })
    .await;

    let session = PtySession::connect(client(&base_url), "pty_1".to_string(), 4, 20)
        .await
        .unwrap();
    session.resize(30, 100).await;
    let message = tokio::time::timeout(Duration::from_secs(5), message_rx)
        .await
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&message).unwrap();

    assert_eq!(value["type"], "resize");
    assert_eq!(value["rows"], 30);
    assert_eq!(value["cols"], 100);
}
