use crate::{db, hub, AppState};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skypeline_shared::{
    AddGroupMemberRequest, AuthResponse, ChannelInfo, CreateGroupRequest, EditMessageRequest,
    LoginRequest, MessageDto, RegisterRequest, RegisterResponse, ResendCodeRequest, StatusResponse,
    VerifyCodeRequest, WsServerMsg,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) type ApiError = (StatusCode, String);

const DEFAULT_NEON_AUTH_ORIGIN: &str = "https://skypeline-co.onrender.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub email: String,
    pub exp: usize,
}

#[derive(Debug)]
struct NeonAuthUser {
    id: String,
    name: String,
    email_verified: bool,
}

pub fn issue_jwt(
    secret: &str,
    user_id: i64,
    username: &str,
    email: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = unix_now() as usize + 60 * 60 * 24 * 14; // 14 days
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        email: email.to_string(),
        exp,
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

pub(crate) fn bearer(headers: &HeaderMap, secret: &str) -> Result<Claims, ApiError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    verify_jwt(secret, raw).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "invalid or expired token".to_string(),
        )
    })
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
        email_verified: user
            .get("emailVerified")
            .and_then(Value::as_bool)
            .unwrap_or(false),
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

/// `POST /api/register` — Neon Auth creates the account and sends an email OTP.
/// No Skypeline session is issued until that code is verified.
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, ApiError> {
    let email = req.email.trim().to_ascii_lowercase();
    let username = req.username.trim().to_string();
    validate_identity(&email, &username, &req.password)?;

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

    db::create_neon_user(&state.db, &user.id, &email, &username)
        .await
        .map_err(db_error)?;

    if !user.email_verified {
        if let Err((status, msg)) = neon_post(
            &state,
            "/email-otp/send-verification-otp",
            json!({ "email": email, "type": "email-verification" }),
        )
        .await
        {
            tracing::warn!("send-verification-otp on register ({status}): {msg}");
        }
    }

    Ok(Json(RegisterResponse {
        email,
        verification_required: true,
        message: "Account created. Enter the 6-digit code sent to your email (check spam too)."
            .to_string(),
    }))
}

/// `POST /api/verify-code` — verifies Neon Auth's emailed code and signs in.
pub async fn verify_code(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyCodeRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
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

    neon_post(
        &state,
        "/email-otp/verify-email",
        json!({ "email": email, "otp": code }),
    )
    .await?;

    let user = db::find_user_by_email(&state.db, &email)
        .await
        .map_err(internal)?
        .ok_or((
            StatusCode::FORBIDDEN,
            "no Wireline account exists for this email".to_string(),
        ))?;
    let token =
        issue_jwt(&state.jwt_secret, user.id, &user.username, &user.email).map_err(internal)?;
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
    Json(req): Json<ResendCodeRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
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

/// `POST /api/login` — validates credentials against Neon Auth and issues the
/// Skypeline relay token only for verified accounts.
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    let email = req.email.trim().to_ascii_lowercase();
    if email.len() < 3 || !email.contains('@') {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }

    let response = match neon_post(
        &state,
        "/sign-in/email",
        json!({ "email": email, "password": req.password }),
    )
    .await
    {
        Ok(resp) => resp,
        Err((status, msg)) => {
            let lower = msg.to_ascii_lowercase();
            if lower.contains("verif") {
                let _ = neon_post(
                    &state,
                    "/email-otp/send-verification-otp",
                    json!({ "email": email, "type": "email-verification" }),
                )
                .await;
                return Err((
                    StatusCode::FORBIDDEN,
                    "Email not verified. We sent a new 6-digit code — enter it to continue."
                        .to_string(),
                ));
            }
            return Err((status, msg));
        }
    };
    let neon_user = parse_neon_user(&response)?;
    if !neon_user.email_verified {
        let _ = neon_post(
            &state,
            "/email-otp/send-verification-otp",
            json!({ "email": email, "type": "email-verification" }),
        )
        .await;
        return Err((
            StatusCode::FORBIDDEN,
            "Email not verified. We sent a new 6-digit code — enter it to continue.".to_string(),
        ));
    }

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

    let token =
        issue_jwt(&state.jwt_secret, user.id, &user.username, &user.email).map_err(internal)?;
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
