//! Small REST client for the server's authenticated JSON API.

use serde::Serialize;
use skypeline_shared::{
    AddGroupMemberRequest, AuthResponse, AvatarInfo, AvatarUpload, ChannelInfo, CreateGroupRequest,
    EditMessageRequest, FriendInfo, FriendRequest, FriendRespondRequest, LoginRequest, MessageDto,
    RegisterRequest, RegisterResponse, ResendCodeRequest, StatusResponse, VerifyCodeRequest,
};

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
        let resp = req
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

    async fn patch<T: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
        token: &str,
    ) -> Result<R, String> {
        let resp = self
            .http
            .patch(format!("{}{}", self.base, path))
            .bearer_auth(token)
            .json(body)
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

    async fn delete<R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        token: &str,
    ) -> Result<R, String> {
        let resp = self
            .http
            .delete(format!("{}{}", self.base, path))
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

    pub async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse, String> {
        self.post("/api/register", req, None).await
    }

    pub async fn verify_code(&self, req: &VerifyCodeRequest) -> Result<AuthResponse, String> {
        self.post("/api/verify-code", req, None).await
    }

    pub async fn resend_code(&self, req: &ResendCodeRequest) -> Result<StatusResponse, String> {
        self.post("/api/resend-code", req, None).await
    }

    pub async fn channels(&self, token: &str) -> Result<Vec<ChannelInfo>, String> {
        self.get("/api/channels", token).await
    }

    pub async fn messages(
        &self,
        token: &str,
        channel_id: i64,
        before_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<MessageDto>, String> {
        let path = match before_id {
            Some(before) => format!("/api/messages/{channel_id}?before={before}&limit={limit}"),
            None => format!("/api/messages/{channel_id}?limit={limit}"),
        };
        self.get(&path, token).await
    }

    pub async fn edit_message(
        &self,
        token: &str,
        message_id: i64,
        content: &str,
    ) -> Result<StatusResponse, String> {
        self.patch(
            &format!("/api/messages/{message_id}"),
            &EditMessageRequest {
                content: content.to_string(),
            },
            token,
        )
        .await
    }

    pub async fn delete_message(
        &self,
        token: &str,
        message_id: i64,
    ) -> Result<StatusResponse, String> {
        self.delete(&format!("/api/messages/{message_id}"), token)
            .await
    }

    pub async fn open_dm(&self, token: &str, user_id: i64) -> Result<ChannelInfo, String> {
        self.post(&format!("/api/dm/{user_id}"), &(), Some(token))
            .await
    }

    pub async fn create_group(
        &self,
        token: &str,
        request: &CreateGroupRequest,
    ) -> Result<ChannelInfo, String> {
        self.post("/api/groups", request, Some(token)).await
    }

    pub async fn add_group_member(
        &self,
        token: &str,
        channel_id: i64,
        request: &AddGroupMemberRequest,
    ) -> Result<ChannelInfo, String> {
        self.post(
            &format!("/api/groups/{channel_id}/members"),
            request,
            Some(token),
        )
        .await
    }

    pub async fn friends(&self, token: &str) -> Result<Vec<FriendInfo>, String> {
        self.get("/api/friends", token).await
    }

    pub async fn friend_request(
        &self,
        token: &str,
        request: &FriendRequest,
    ) -> Result<StatusResponse, String> {
        self.post("/api/friends/request", request, Some(token))
            .await
    }

    pub async fn friend_respond(
        &self,
        token: &str,
        request: &FriendRespondRequest,
    ) -> Result<StatusResponse, String> {
        self.post("/api/friends/respond", request, Some(token))
            .await
    }

    pub async fn get_avatar(&self, token: &str, user_id: i64) -> Result<AvatarInfo, String> {
        self.get(&format!("/api/avatar/{user_id}"), token).await
    }

    #[allow(dead_code)]
    pub async fn upload_avatar(
        &self,
        token: &str,
        upload: &AvatarUpload,
    ) -> Result<AvatarInfo, String> {
        self.post("/api/avatar", upload, Some(token)).await
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
