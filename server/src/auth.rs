use crate::{db, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use skypeline_shared::{
    AuthResponse, ChannelInfo, LoginRequest, MessageDto, RegisterRequest, RegisterResponse,
    ResendCodeRequest, StatusResponse, VerifyCodeRequest,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

type ApiError = (StatusCode, String);

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

fn bearer(headers: &HeaderMap, secret: &str) -> Result<Claims, ApiError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing bearer token".to_string(),
        ))?;
    verify_jwt(secret, raw)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token".to_string()))
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
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
    let url = format!(
        "{}{}",
        state.neon_auth_base_url.trim_end_matches('/'),
        path
    );
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
        let mapped = if status == 409 || lower.contains("already exists") || lower.contains("already registered") {
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
    let user = value
        .get("user")
        .and_then(Value::as_object)
        .ok_or((
            StatusCode::BAD_GATEWAY,
            "auth service returned an invalid user".to_string(),
        ))?;

    let get_str = |key: &str| user.get(key).and_then(Value::as_str).unwrap_or("").to_string();
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

    Ok(Json(RegisterResponse {
        email,
        verification_required: true,
        message: "Account created. Enter the confirmation code sent to your email.".to_string(),
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
        return Err((StatusCode::BAD_REQUEST, "enter the code from your email".to_string()));
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
            "no Skypeline account exists for this email".to_string(),
        ))?;
    let token = issue_jwt(&state.jwt_secret, user.id, &user.username, &user.email)
        .map_err(internal)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
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

    let response = neon_post(
        &state,
        "/sign-in/email",
        json!({ "email": email, "password": req.password }),
    )
    .await?;
    let neon_user = parse_neon_user(&response)?;
    if !neon_user.email_verified {
        return Err((
            StatusCode::FORBIDDEN,
            "confirm your email before signing in".to_string(),
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

    let token = issue_jwt(&state.jwt_secret, user.id, &user.username, &user.email)
        .map_err(internal)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
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
        return Ok(db::dm_members(&ch.name).is_some_and(|(a, b)| a == user_id || b == user_id));
    }
    Ok(ch.kind == "text")
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(channel_id): Path<i64>,
) -> Result<Json<Vec<MessageDto>>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    if !can_access_channel(&state, claims.sub, channel_id)
        .await
        .map_err(internal)?
    {
        return Err((StatusCode::FORBIDDEN, "no access to channel".to_string()));
    }
    let messages = db::list_messages(&state.db, channel_id, 100)
        .await
        .map_err(internal)?;
    Ok(Json(messages))
}

/// `POST /api/dm/{user_id}` — open (or create) a direct-message channel with another user.
pub async fn open_dm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(peer_id): Path<i64>,
) -> Result<Json<ChannelInfo>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    if peer_id == claims.sub {
        return Err((
            StatusCode::BAD_REQUEST,
            "cannot DM yourself".to_string(),
        ));
    }
    let peer_name = db::find_username(&state.db, peer_id)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?;
    let id = db::get_or_create_dm(&state.db, claims.sub, peer_id)
        .await
        .map_err(internal)?;
    Ok(Json(ChannelInfo {
        id,
        name: peer_name,
        kind: "dm".to_string(),
        peer_user_id: Some(peer_id),
    }))
}

pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
