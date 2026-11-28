use crate::{db, hub, AppState};
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use dashmap::DashMap;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skypeline_shared::{
    AddGroupMemberRequest, AuthResponse, ChannelInfo, CreateGroupRequest, EditMessageRequest,
    LoginRequest, MessageDto, RegisterRequest, RegisterResponse, ResendCodeRequest, StatusResponse,
    VerifyCodeRequest, WsServerMsg,
};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) type ApiError = (StatusCode, String);

const DEFAULT_NEON_AUTH_ORIGIN: &str = "https://skypeline-co.onrender.com";

/// Per-IP and per-(IP, email) sliding-window brute-force protector.
pub struct IpGuard {
    attempts: DashMap<String, Vec<Instant>>,
}

impl IpGuard {
    pub fn new() -> Self {
        Self {
            attempts: DashMap::new(),
        }
    }

    pub fn check(&self, key: &str, max_attempts: usize, window_secs: u64) -> Result<(), ApiError> {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        if let Some(mut entry) = self.attempts.get_mut(key) {
            entry.retain(|t| now.duration_since(*t) < window);
            if entry.len() >= max_attempts {
                let oldest = entry.first().copied().unwrap_or(now);
                let retry_secs = window
                    .saturating_sub(now.duration_since(oldest))
                    .as_secs()
                    .max(1);
                let mins = (retry_secs + 59) / 60;
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    format!(
                        "too many attempts from this IP — try again in {mins} minute{}",
                        if mins == 1 { "" } else { "s" }
                    ),
                ));
            }
        }
        Ok(())
    }

    pub fn record(&self, key: &str, window_secs: u64) {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let mut entry = self.attempts.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < window);
        entry.push(now);
        // Cap map size defensively against memory exhaustion.
        if self.attempts.len() > 20_000 {
            self.attempts.clear();
        }
    }

    pub fn clear(&self, key: &str) {
        self.attempts.remove(key);
    }
}

/// Extracts the client IP from reverse-proxy headers (Cloudflare / Render / Nginx),
/// falling back to the TCP peer address when present.
pub fn extract_forwarded_ip(headers: &HeaderMap) -> Option<String> {
    for header in ["cf-connecting-ip", "x-real-ip", "x-forwarded-for"] {
        if let Some(val) = headers.get(header).and_then(|v| v.to_str().ok()) {
            if let Some(first) = val.split(',').next().map(str::trim) {
                if first.parse::<IpAddr>().is_ok() {
                    return Some(first.to_string());
                }
            }
        }
    }
    None
}

pub fn extract_client_ip(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    extract_forwarded_ip(headers)
        .or_else(|| peer.map(|p| p.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Computes a privacy-preserving keyed IP subnet fingerprint (IPv4 /24, IPv6 /48)
/// so stolen JWTs cannot be replayed from another network while tolerating minor ISP hops.
pub fn ip_fingerprint(secret: &str, ip: &str) -> String {
    let prefix = match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => {
            let [a, b, c, _] = v4.octets();
            format!("v4:{a}.{b}.{c}")
        }
        Ok(IpAddr::V6(v6)) => {
            let seg = v6.segments();
            format!("v6:{:x}:{:x}:{:x}", seg[0], seg[1], seg[2])
        }
        Err(_) => format!("raw:{ip}"),
    };
    let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
    let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
    for &b in secret.as_bytes().iter().chain(b":ip:").chain(prefix.as_bytes()) {
        h1 ^= u64::from(b);
        h1 = h1.wrapping_mul(0x0100_0000_01b3);
        h2 = h2.rotate_left(7) ^ u64::from(b);
        h2 = h2.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
    format!("{h1:016x}{h2:016x}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub email: String,
    pub exp: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_fp: Option<String>,
}

#[derive(Debug)]
struct NeonAuthUser {
    id: String,
    name: String,
}

pub fn issue_jwt(
    secret: &str,
    user_id: i64,
    username: &str,
    email: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    issue_jwt_with_ip(secret, user_id, username, email, None)
}

pub fn issue_jwt_with_ip(
    secret: &str,
    user_id: i64,
    username: &str,
    email: &str,
    client_ip: Option<&str>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = unix_now() as usize + 60 * 60 * 24 * 14; // 14 days
    let ip_fp = client_ip
        .filter(|ip| *ip != "unknown" && !ip.is_empty())
        .map(|ip| ip_fingerprint(secret, ip));
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        email: email.to_string(),
        exp,
        ip_fp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_jwt(secret: &str, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
}

/// Validates that an IP-bound JWT matches the incoming request's client IP subnet.
pub fn verify_ip_binding(claims: &Claims, headers: &HeaderMap, secret: &str) -> Result<(), ApiError> {
    if let Some(expected_fp) = claims.ip_fp.as_deref() {
        if let Some(req_ip) = extract_forwarded_ip(headers) {
            if ip_fingerprint(secret, &req_ip) != expected_fp {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "session bound to a different IP address — please sign in again".to_string(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn bearer(headers: &HeaderMap, secret: &str) -> Result<Claims, ApiError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    let claims = verify_jwt(secret, raw).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "invalid or expired token".to_string(),
        )
    })?;
    verify_ip_binding(&claims, headers, secret)?;
    Ok(claims)
}

pub(crate) fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    tracing::warn!("internal error: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal server error".to_string(),
    )
}

fn status_from_code(code: u16) -> StatusCode {
    match code {
        400 => StatusCode::BAD_REQUEST,
        401 => StatusCode::UNAUTHORIZED,
        403 => StatusCode::FORBIDDEN,
        404 => StatusCode::NOT_FOUND,
        409 => StatusCode::CONFLICT,
        429 => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn auth_error_message(body: &str, status: u16) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        for key in ["message", "error", "description", "code"] {
            if let Some(message) = value.get(key).and_then(Value::as_str) {
                if !message.trim().is_empty() {
                    return message.to_string();
                }
            }
        }
    }
    trim_error(body, status)
}

fn trim_error(body: &str, status: u16) -> String {
    let body = body.trim();
    if body.is_empty() {
        format!("auth service returned HTTP {status}")
    } else if body.len() > 300 {
        format!("auth service: {}...", &body[..300])
    } else {
        format!("auth service: {body}")
    }
}

/// Calls a Managed Better Auth REST endpoint. Skypeline sends a trusted Origin
/// because Neon Auth requires one when no absolute callback URL is supplied.
async fn neon_post(state: &AppState, path: &str, body: Value) -> Result<Value, ApiError> {
    let url = format!("{}{}", state.neon_auth_base_url.trim_end_matches('/'), path);
    let origin = if state.neon_auth_origin.trim().is_empty() {
        DEFAULT_NEON_AUTH_ORIGIN
    } else {
        state.neon_auth_origin.as_str()
    };
    let response = state
        .auth_http
        .post(url)
        .header(axum::http::header::ORIGIN, origin)
        .header(axum::http::header::ACCEPT, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| internal(format!("auth service unavailable: {e}")))?;

    let status = response.status().as_u16();
    let text = response.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        let message = auth_error_message(&text, status);
        let lower = message.to_ascii_lowercase();
        let mapped = if status == 409
            || lower.contains("already exists")
            || lower.contains("already registered")
        {
            409
        } else {
            status
        };
        return Err((status_from_code(mapped), message));
    }
    if text.trim().is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_str(&text).map_err(internal)
    }
}

fn parse_neon_user(value: &Value) -> Result<NeonAuthUser, ApiError> {
    let user = value.get("user").and_then(Value::as_object).ok_or((
        StatusCode::BAD_GATEWAY,
        "auth service returned an invalid user".to_string(),
    ))?;

    let get_str = |key: &str| {
        user.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let id = get_str("id");
    let email = get_str("email");
    if id.is_empty() || email.is_empty() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "auth service returned an incomplete user".to_string(),
        ));
    }

    Ok(NeonAuthUser {
        id,
        name: get_str("name"),
    })
}

fn validate_identity(email: &str, username: &str, password: &str) -> Result<(), ApiError> {
    if email.len() < 3 || !email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "valid email required".to_string()));
    }
    let username_len = username.chars().count();
    if username_len < 2 || username_len > 32 {
        return Err((
            StatusCode::BAD_REQUEST,
            "username must be 2-32 characters".to_string(),
        ));
    }
    if password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "password must be at least 8 characters".to_string(),
        ));
    }
    Ok(())
}

fn db_error(e: sqlx::Error) -> ApiError {
    let unique_violation = e
        .as_database_error()
        .and_then(|d| d.code())
        .is_some_and(|c| c == "23505");
    if unique_violation {
        (
            StatusCode::CONFLICT,
            "an account with this email already exists".to_string(),
        )
    } else {
        internal(e)
    }
}

/// `POST /api/register` — IP rate-limited account registration via Neon Auth.
pub async fn register(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ApiError> {
    let client_ip = extract_client_ip(&headers, Some(peer));
    let reg_key = format!("reg:{client_ip}");
    state.ip_guard.check(&reg_key, 6, 3600)?;

    let email = req.email.trim().to_ascii_lowercase();
    let username = req.username.trim().to_string();
    validate_identity(&email, &username, &req.password)?;

    state.ip_guard.record(&reg_key, 3600);
    let response = neon_post(
        &state,
        "/sign-up/email",
        json!({
            "email": email,
            "password": req.password,
            "name": username,
        }),
    )
    .await?;
    let user = parse_neon_user(&response)?;

    let created = db::create_neon_user(&state.db, &user.id, &email, &username)
        .await
        .map_err(db_error)?;
    let fp = ip_fingerprint(&state.jwt_secret, &client_ip);
    let _ = db::record_trusted_ip(&state.db, created.id, &fp).await;

    Ok(Json(RegisterResponse {
        email,
        verification_required: false,
        message: "Account created! You can sign in now.".to_string(),
    }))
}

/// `POST /api/verify-code` — verifies Neon Auth's emailed code and signs in with IP-bound JWT.
pub async fn verify_code(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<VerifyCodeRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let client_ip = extract_client_ip(&headers, Some(peer));
    let ver_key = format!("ver:{client_ip}");
    state.ip_guard.check(&ver_key, 10, 900)?;

    let email = req.email.trim().to_ascii_lowercase();
    let code = req.code.trim();
    if email.len() < 3 || !email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "valid email required".to_string()));
    }
    if code.len() < 4 || code.len() > 16 {
        return Err((
            StatusCode::BAD_REQUEST,
            "enter the code from your email".to_string(),
        ));
    }

    if let Err(err) = neon_post(
        &state,
        "/email-otp/verify-email",
        json!({ "email": email, "otp": code }),
    )
    .await
    {
        state.ip_guard.record(&ver_key, 900);
        return Err(err);
    }

    let user = db::find_user_by_email(&state.db, &email)
        .await
        .map_err(internal)?
        .ok_or((
            StatusCode::FORBIDDEN,
            "no Wireline account exists for this email".to_string(),
        ))?;
    state.ip_guard.clear(&ver_key);
    let fp = ip_fingerprint(&state.jwt_secret, &client_ip);
    let _ = db::record_trusted_ip(&state.db, user.id, &fp).await;

    let token = issue_jwt_with_ip(
        &state.jwt_secret,
        user.id,
        &user.username,
        &user.email,
        Some(&client_ip),
    )
    .map_err(internal)?;
    let avatar_hash = db::avatar_hash_for_user(&state.db, user.id)
        .await
        .map_err(internal)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
        avatar_hash,
    }))
}

/// `POST /api/resend-code` — asks Neon Auth to email a fresh confirmation code.
pub async fn resend_code(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ResendCodeRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    let client_ip = extract_client_ip(&headers, Some(peer));
    let resend_key = format!("resend:{client_ip}");
    state.ip_guard.check(&resend_key, 6, 1800)?;
    state.ip_guard.record(&resend_key, 1800);

    let email = req.email.trim().to_ascii_lowercase();
    if email.len() < 3 || !email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "valid email required".to_string()));
    }
    neon_post(
        &state,
        "/email-otp/send-verification-otp",
        json!({ "email": email, "type": "email-verification" }),
    )
    .await?;
    Ok(Json(StatusResponse {
        ok: true,
        message: "A new confirmation code is on its way.".to_string(),
    }))
}

/// `POST /api/login` — IP rate-limited login with IP-bound JWT sessions and new-IP alerts.
pub async fn login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let client_ip = extract_client_ip(&headers, Some(peer));
    let ip_key = format!("login-ip:{client_ip}");
    state.ip_guard.check(&ip_key, 10, 900)?;

    let email = req.email.trim().to_ascii_lowercase();
    if email.len() < 3 || !email.contains('@') {
        state.ip_guard.record(&ip_key, 900);
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }
    let pair_key = format!("login-pair:{client_ip}:{email}");
    state.ip_guard.check(&pair_key, 5, 900)?;

    let response = match neon_post(
        &state,
        "/sign-in/email",
        json!({ "email": email, "password": req.password }),
    )
    .await
    {
        Ok(res) => res,
        Err(err) => {
            state.ip_guard.record(&ip_key, 900);
            state.ip_guard.record(&pair_key, 900);
            return Err(err);
        }
    };
    let neon_user = parse_neon_user(&response)?;

    let user = match db::find_user_by_neon_id(&state.db, &neon_user.id)
        .await
        .map_err(internal)?
    {
        Some(user) => user,
        None => match db::find_user_by_email(&state.db, &email)
            .await
            .map_err(internal)?
        {
            Some(existing) => db::link_neon_user(&state.db, existing.id, &neon_user.id)
                .await
                .map_err(db_error)?
                .ok_or_else(|| {
                    (
                        StatusCode::CONFLICT,
                        "this email is linked to a different account".to_string(),
                    )
                })?,
            None => {
                let username = if neon_user.name.trim().is_empty() {
                    email.split('@').next().unwrap_or("User")
                } else {
                    neon_user.name.trim()
                };
                db::create_neon_user(&state.db, &neon_user.id, &email, username)
                    .await
                    .map_err(db_error)?
            }
        },
    };

    state.ip_guard.clear(&pair_key);
    let fp = ip_fingerprint(&state.jwt_secret, &client_ip);
    if let Ok(is_new_ip) = db::record_trusted_ip(&state.db, user.id, &fp).await {
        if is_new_ip {
            tracing::info!("new IP login recorded for user_id={}", user.id);
            state.hub.send(
                hub::Scope::Users(vec![user.id]),
                None,
                &WsServerMsg::Error {
                    message: "Security alert: your account was just signed into from a new IP address."
                        .to_string(),
                },
            );
        }
    }

    let token = issue_jwt_with_ip(
        &state.jwt_secret,
        user.id,
        &user.username,
        &user.email,
        Some(&client_ip),
    )
    .map_err(internal)?;
    let avatar_hash = db::avatar_hash_for_user(&state.db, user.id)
        .await
        .map_err(internal)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
        avatar_hash,
    }))
}

pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ChannelInfo>>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let channels = db::list_channels_for_user(&state.db, claims.sub)
        .await
        .map_err(internal)?;
    Ok(Json(channels))
}

/// Returns `Ok(true)` if `user_id` may read/write `channel_id`.
pub async fn can_access_channel(
    state: &AppState,
    user_id: i64,
    channel_id: i64,
) -> Result<bool, sqlx::Error> {
    let Some(ch) = db::get_channel(&state.db, channel_id).await? else {
        return Ok(false);
    };
    if ch.kind == "dm" {
        let Some((a, b)) = db::dm_members(&ch.name) else {
            return Ok(false);
        };
        return Ok((a == user_id || b == user_id) && db::are_friends(&state.db, a, b).await?);
    }
    if ch.kind == "group" {
        return db::is_channel_member(&state.db, channel_id, user_id).await;
    }
    Ok(false)
}

#[derive(Deserialize)]
pub struct MessagesQuery {
    pub before: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel_id): Path<i64>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<Vec<MessageDto>>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    if !can_access_channel(&state, claims.sub, channel_id)
        .await
        .map_err(internal)?
    {
        return Err((StatusCode::FORBIDDEN, "no access to channel".to_string()));
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let messages = db::list_messages(&state.db, channel_id, query.before, limit)
        .await
        .map_err(internal)?;
    Ok(Json(messages))
}

pub async fn edit_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(message_id): Path<i64>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let trimmed = req.content.trim();
    if trimmed.is_empty() || trimmed.len() > 4000 {
        return Err((
            StatusCode::BAD_REQUEST,
            "message must be 1-4000 characters".to_string(),
        ));
    }
    let Some((channel_id, edited_at)) =
        db::edit_message(&state.db, message_id, claims.sub, trimmed)
            .await
            .map_err(internal)?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            "message not found or not owned by you".to_string(),
        ));
    };

    if let Some(scope) = hub::channel_scope(&state, claims.sub, channel_id).await {
        state.hub.send(
            scope,
            None,
            &WsServerMsg::MessageEdited {
                channel_id,
                message_id,
                content: trimmed.to_string(),
                edited_at,
            },
        );
    }

    Ok(Json(StatusResponse {
        ok: true,
        message: "Message updated.".to_string(),
    }))
}

pub async fn delete_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(message_id): Path<i64>,
) -> Result<Json<StatusResponse>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let Some(channel_id) = db::delete_message(&state.db, message_id, claims.sub)
        .await
        .map_err(internal)?
    else {
        return Err((
            StatusCode::NOT_FOUND,
            "message not found or not owned by you".to_string(),
        ));
    };

    if let Some(scope) = hub::channel_scope(&state, claims.sub, channel_id).await {
        state.hub.send(
            scope,
            None,
            &WsServerMsg::MessageDeleted {
                channel_id,
                message_id,
            },
        );
    }

    Ok(Json(StatusResponse {
        ok: true,
        message: "Message deleted.".to_string(),
    }))
}

/// `POST /api/dm/{user_id}` — open (or create) a direct-message channel with another user.
pub async fn open_dm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(peer_id): Path<i64>,
) -> Result<Json<ChannelInfo>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    if peer_id == claims.sub {
        return Err((StatusCode::BAD_REQUEST, "cannot DM yourself".to_string()));
    }
    if !db::are_friends(&state.db, claims.sub, peer_id)
        .await
        .map_err(internal)?
    {
        return Err((
            StatusCode::FORBIDDEN,
            "you must be friends before messaging".to_string(),
        ));
    }
    let peer_name = db::find_username(&state.db, peer_id)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?;
    let id = db::get_or_create_dm(&state.db, claims.sub, peer_id)
        .await
        .map_err(internal)?;
    state.hub.send(
        hub::Scope::Users(vec![peer_id]),
        None,
        &WsServerMsg::ChannelsUpdated,
    );
    let members = db::channel_members(&state.db, id).await.map_err(internal)?;
    Ok(Json(ChannelInfo {
        id,
        name: peer_name,
        kind: "dm".to_string(),
        peer_user_id: Some(peer_id),
        members,
    }))
}

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<ChannelInfo>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let name = req.name.trim();
    if name.len() < 2 || name.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            "group name must be 2-64 characters".to_string(),
        ));
    }
    let mut member_ids = req.member_ids;
    member_ids.retain(|id| *id != claims.sub);
    member_ids.sort_unstable();
    member_ids.dedup();
    for id in &member_ids {
        if !db::are_friends(&state.db, claims.sub, *id)
            .await
            .map_err(internal)?
        {
            return Err((
                StatusCode::FORBIDDEN,
                "you can only add friends to a group".to_string(),
            ));
        }
    }
    let id = db::create_group(&state.db, claims.sub, name, &member_ids)
        .await
        .map_err(internal)?;
    if !member_ids.is_empty() {
        state.hub.send(
            hub::Scope::Users(member_ids),
            None,
            &WsServerMsg::ChannelsUpdated,
        );
    }
    let members = db::channel_members(&state.db, id).await.map_err(internal)?;
    Ok(Json(ChannelInfo {
        id,
        name: name.to_string(),
        kind: "group".to_string(),
        peer_user_id: None,
        members,
    }))
}

pub async fn add_group_member(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel_id): Path<i64>,
    Json(req): Json<AddGroupMemberRequest>,
) -> Result<Json<ChannelInfo>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let Some(ch) = db::get_channel(&state.db, channel_id)
        .await
        .map_err(internal)?
    else {
        return Err((StatusCode::NOT_FOUND, "group not found".to_string()));
    };
    if ch.kind != "group" {
        return Err((StatusCode::BAD_REQUEST, "not a group chat".to_string()));
    }
    if !db::is_channel_member(&state.db, channel_id, claims.sub)
        .await
        .map_err(internal)?
    {
        return Err((StatusCode::FORBIDDEN, "no access to group".to_string()));
    }
    if !db::are_friends(&state.db, claims.sub, req.user_id)
        .await
        .map_err(internal)?
    {
        return Err((
            StatusCode::FORBIDDEN,
            "you can only add friends to a group".to_string(),
        ));
    }
    db::add_group_member(&state.db, channel_id, req.user_id)
        .await
        .map_err(internal)?;
    state.hub.send(
        hub::Scope::Users(vec![req.user_id]),
        None,
        &WsServerMsg::ChannelsUpdated,
    );
    let members = db::channel_members(&state.db, channel_id)
        .await
        .map_err(internal)?;
    Ok(Json(ChannelInfo {
        id: channel_id,
        name: ch.name,
        kind: "group".to_string(),
        peer_user_id: None,
        members,
    }))
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_issue_and_verify_round_trip() {
        let secret = "super_secret_test_key_123456";
        let token = issue_jwt(secret, 7, "lukas", "lukas@example.com").expect("issue jwt");
        let claims = verify_jwt(secret, &token).expect("verify jwt");
        assert_eq!(claims.sub, 7);
        assert_eq!(claims.username, "lukas");
        assert_eq!(claims.email, "lukas@example.com");
        assert!(verify_jwt("wrong_secret_1234567890", &token).is_err());
    }
}
