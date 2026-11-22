//! Small REST client for the server's authenticated JSON API.

use serde::Serialize;
use skypeline_shared::{AuthResponse, ChannelInfo, LoginRequest, MessageDto, RegisterRequest};

#[derive(Clone)]
pub struct ApiClient {
    pub base: String,
    http: reqwest::Client,
}

impl ApiClient {
    pub fn new(base: String) -> Self {
        Self {
            base,
            http: reqwest::Client::builder()
                .use_rustls_tls()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    async fn post<T: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
        token: Option<&str>,
    ) -> Result<R, String> {
        let mut req = self.http.post(format!("{}{}", self.base, path)).json(body);
        if let Some(token) = token {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.map_err(|e| format!("network error: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(trim_error(&text, status.as_u16()));
        }
        serde_json::from_str(&text).map_err(|e| format!("invalid server response: {e}"))
    }

    async fn get<R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        token: &str,
    ) -> Result<R, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base, path))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("network error: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(trim_error(&text, status.as_u16()));
        }
        serde_json::from_str(&text).map_err(|e| format!("invalid server response: {e}"))
    }

    pub async fn login(&self, req: &LoginRequest) -> Result<AuthResponse, String> {
        self.post("/api/login", req, None).await
    }

    pub async fn register(&self, req: &RegisterRequest) -> Result<AuthResponse, String> {
        self.post("/api/register", req, None).await
    }

    pub async fn channels(&self, token: &str) -> Result<Vec<ChannelInfo>, String> {
        self.get("/api/channels", token).await
    }

    pub async fn messages(&self, token: &str, channel_id: i64) -> Result<Vec<MessageDto>, String> {
        self.get(&format!("/api/messages/{channel_id}"), token).await
    }

    pub async fn open_dm(&self, token: &str, user_id: i64) -> Result<ChannelInfo, String> {
        self.post(&format!("/api/dm/{user_id}"), &(), Some(token)).await
    }
}

fn trim_error(body: &str, status: u16) -> String {
    let body = body.trim();
    if body.is_empty() {
        format!("server returned HTTP {status}")
    } else if body.len() > 300 {
        format!("HTTP {status}: {}...", &body[..300])
    } else {
        format!("HTTP {status}: {body}")
    }
}
