use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Config {
    pub server_url: Option<String>,
    pub token: Option<String>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        config_path()
    }

    pub fn load() -> Self {
        Self::load_from_path(Self::config_path())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to_path(Self::config_path())
    }

    pub fn is_configured(&self) -> bool {
        self.server_url
            .as_deref()
            .is_some_and(|server_url| !server_url.trim().is_empty())
    }

    fn load_from_path(path: PathBuf) -> Self {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    fn save_to_path(&self, path: PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = std::env::var_os("WARP_OPENCODE_CONFIG_PATH") {
        return PathBuf::from(path);
    }

    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("warp-opencode").join("config.json")
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_config_is_not_configured() {
        assert!(!Config::default().is_configured());
    }

    #[test]
    fn empty_server_url_is_not_configured() {
        let config = Config {
            server_url: Some("  ".to_string()),
            token: None,
        };

        assert!(!config.is_configured());
    }

    #[test]
    fn non_empty_server_url_is_configured() {
        let config = Config {
            server_url: Some("https://opencode.example.com".to_string()),
            token: None,
        };

        assert!(config.is_configured());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = std::env::temp_dir().join(format!(
            "warp-opencode-config-test-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        std::env::set_var("WARP_OPENCODE_CONFIG_PATH", &path);

        let config = Config {
            server_url: Some("https://opencode.example.com".to_string()),
            token: Some("secret".to_string()),
        };

        config.save().unwrap();
        assert_eq!(Config::load(), config);

        let _ = std::fs::remove_file(path);
        std::env::remove_var("WARP_OPENCODE_CONFIG_PATH");
    }
}
