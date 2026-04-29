use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone)]
pub enum Auth {
    None,
    Basic { username: String, password: String },
}

impl Auth {
    pub fn basic_token(&self) -> Option<String> {
        match self {
            Auth::None => None,
            Auth::Basic { username, password } => Some(
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}")),
            ),
        }
    }

    fn header_value(&self) -> Result<Option<HeaderValue>, ApiError> {
        let Some(token) = self.basic_token() else {
            return Ok(None);
        };
        HeaderValue::from_str(&format!("Basic {token}"))
            .map(Some)
            .map_err(|err| ApiError::InvalidHeader(err.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: Url,
    pub auth: Auth,
}

impl ApiConfig {
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, ApiError> {
        let base_url = Url::parse(base_url.as_ref())?;
        Ok(Self {
            base_url,
            auth: Auth::None,
        })
    }
}

#[derive(Clone)]
pub struct ApiClient {
    http: reqwest::Client,
    config: ApiConfig,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("invalid url: {0}")]
    Url(#[from] url::ParseError),
    #[error("http request failed: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("json decode failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid auth header: {0}")]
    InvalidHeader(String),
    #[error("server returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl ApiClient {
    pub fn new(config: ApiConfig) -> Result<Self, ApiError> {
        let mut headers = HeaderMap::new();
        if let Some(auth) = config.auth.header_value()? {
            headers.insert(AUTHORIZATION, auth);
        }
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self { http, config })
    }

    pub fn config(&self) -> &ApiConfig {
        &self.config
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn url(&self, path: &str) -> Result<Url, ApiError> {
        let path = path.trim_start_matches('/');
        Ok(self.config.base_url.join(path)?)
    }

    pub fn websocket_url(&self, path: &str) -> Result<Url, ApiError> {
        let mut url = self.url(path)?;
        match url.scheme() {
            "http" => url.set_scheme("ws").ok(),
            "https" => url.set_scheme("wss").ok(),
            _ => None,
        };
        if let Some(token) = self.config.auth.basic_token() {
            url.query_pairs_mut().append_pair("auth_token", &token);
        }
        Ok(url)
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.decode(self.http.get(self.url(path)?).send().await?)
            .await
    }

    pub async fn get_or_default<T: DeserializeOwned + Default>(
        &self,
        path: &str,
    ) -> Result<T, ApiError> {
        self.decode_or_default(self.http.get(self.url(path)?).send().await?)
            .await
    }

    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.decode(self.http.delete(self.url(path)?).send().await?)
            .await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        self.decode(self.http.post(self.url(path)?).json(body).send().await?)
            .await
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.decode(self.http.post(self.url(path)?).send().await?)
            .await
    }

    pub async fn patch<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        self.decode(self.http.patch(self.url(path)?).json(body).send().await?)
            .await
    }

    pub async fn put<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        self.decode(self.http.put(self.url(path)?).json(body).send().await?)
            .await
    }

    async fn decode<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, ApiError> {
        let status = response.status();
        let body = response.text().await?;
        tracing::debug!(status = %status, body_len = body.len(), "api response");
        if !status.is_success() {
            return Err(ApiError::Status { status, body });
        }
        Ok(serde_json::from_str(&body)?)
    }

    async fn decode_or_default<T: DeserializeOwned + Default>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, ApiError> {
        let status = response.status();
        let body = response.text().await?;
        tracing::debug!(status = %status, body_len = body.len(), "api response");
        if !status.is_success() {
            return Err(ApiError::Status { status, body });
        }
        if body.trim().is_empty() {
            return Ok(T::default());
        }
        Ok(serde_json::from_str(&body)?)
    }
}
