use anyhow::Result;
use std::borrow::Cow;
use warp_opencode::api::{ApiClient, ApiConfig, Auth};
use warp_opencode::state::{AppStore, ConnectionStatus};
use warp_opencode::views::RootView;
use warpui::{platform, AssetProvider};

#[derive(Clone, Copy)]
struct EmptyAssets;

impl AssetProvider for EmptyAssets {
    fn get(&self, path: &str) -> Result<Cow<'_, [u8]>> {
        anyhow::bail!("no embedded asset exists at {path}")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut config = ApiConfig::new(&args.base_url())?;
    if let Some(password) = args.password {
        config.auth = Auth::Basic {
            username: args.username.unwrap_or_else(|| "opencode".to_string()),
            password,
        };
    }
    let client = ApiClient::new(config)?;
    let store = AppStore::default();
    bootstrap(client.clone(), store.clone()).await;

    let app_builder = platform::AppBuilder::new(
        platform::AppCallbacks::default(),
        Box::new(EmptyAssets),
        None,
    );
    let _ = app_builder.run(move |ctx| {
        let store = store.clone();
        ctx.add_window(warpui::AddWindowOptions::default(), move |ctx| {
            RootView::new(ctx, store.clone())
        });
    });
    Ok(())
}

async fn bootstrap(client: ApiClient, store: AppStore) {
    store.set_connection(ConnectionStatus::Connecting).await;
    let result = async {
        let _ = client.health().await?;
        let sessions = client.list_sessions().await?;
        let statuses = client.session_status().await.unwrap_or_default();
        let permissions = client.list_permissions().await.unwrap_or_default();
        let questions = client.list_questions().await.unwrap_or_default();
        let providers = client.list_providers().await.ok();
        store
            .replace_bootstrap(sessions, statuses, permissions, questions, providers)
            .await;
        Ok::<(), warp_opencode::api::ApiError>(())
    }
    .await;
    match result {
        Ok(()) => store.set_connection(ConnectionStatus::Connected).await,
        Err(err) => {
            store
                .set_connection(ConnectionStatus::Error(err.to_string()))
                .await
        }
    }
}

struct Args {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
}

impl Args {
    fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut parsed = Self {
            host: "localhost".to_string(),
            port: 4096,
            username: None,
            password: None,
        };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => parsed.host = args.next().unwrap_or(parsed.host),
                "--port" => {
                    parsed.port = args
                        .next()
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(parsed.port)
                }
                "--username" => parsed.username = args.next(),
                "--password" => parsed.password = args.next(),
                "--session" => {
                    let _ = args.next();
                }
                _ => {}
            }
        }
        parsed
    }

    fn base_url(&self) -> String {
        if self.host.starts_with("http://") || self.host.starts_with("https://") {
            format!("{}", self.host.trim_end_matches('/'))
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }
}
