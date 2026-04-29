use anyhow::Result;
use std::borrow::Cow;
use warp_opencode::config::Config;
use warp_opencode::views::onboarding::OnboardingView;
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
    let mut config = Config::load();
    if let Some(server_url) = args.server_url_override() {
        config.server_url = Some(server_url);
    }
    if let Some(token) = args.token.clone().or(args.password.clone()) {
        config.token = Some(token);
    }
    let username = args.username.unwrap_or_else(|| "opencode".to_string());

    let app_builder = platform::AppBuilder::new(
        platform::AppCallbacks::default(),
        Box::new(EmptyAssets),
        None,
    );
    let _ = app_builder.run(move |ctx| {
        let config = config.clone();
        let username = username.clone();
        ctx.add_window(warpui::AddWindowOptions::default(), move |ctx| {
            OnboardingView::new(ctx, config.clone(), username.clone())
        });
    });
    Ok(())
}

struct Args {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    token: Option<String>,
}

impl Args {
    fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut parsed = Self {
            host: None,
            port: None,
            username: None,
            password: None,
            token: None,
        };
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => parsed.host = args.next(),
                "--server-url" => parsed.host = args.next(),
                "--port" => parsed.port = args.next().and_then(|value| value.parse().ok()),
                "--username" => parsed.username = args.next(),
                "--password" => parsed.password = args.next(),
                "--token" => parsed.token = args.next(),
                "--session" => {
                    let _ = args.next();
                }
                _ => {}
            }
        }
        parsed
    }

    fn server_url_override(&self) -> Option<String> {
        let host = self.host.as_deref()?;
        if host.starts_with("http://") || host.starts_with("https://") {
            Some(host.trim_end_matches('/').to_string())
        } else {
            Some(format!("http://{}:{}", host, self.port.unwrap_or(4096)))
        }
    }
}
