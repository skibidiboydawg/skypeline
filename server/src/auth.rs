use crate::{db, AppState};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use skypeline_shared::{AuthResponse, ChannelInfo, LoginRequest, MessageDto, RegisterRequest};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

type ApiError = (StatusCode, String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub email: String,
    pub exp: usize,
}

pub fn issue_jwt(secret: &str, user_id: i64, username: &str, email: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = unix_now() as usize + 60 * 60 * 24 * 14; // 14 days
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        email: email.to_string(),
        exp,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

pub fn verify_jwt(secret: &str, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|d| d.claims)
}

fn bearer(headers: &HeaderMap, secret: &str) -> Result<Claims, ApiError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    verify_jwt(secret, raw).map_err(|_| (StatusCode::UNAUTHORIZED, "invalid or expired token".to_string()))
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    tracing::warn!("internal error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
}

pub async fn register(State(state): State<Arc<AppState>>, Json(req): Json<RegisterRequest>) -> Result<Json<AuthResponse>, ApiError> {
    let email = req.email.trim().to_ascii_lowercase();
    let username = req.username.trim().to_string();
    if email.len() < 3 || !email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "valid email required".to_string()));
    }
    if username.chars().count() < 2 || username.chars().count() > 32 {
        return Err((StatusCode::BAD_REQUEST, "username must be 2-32 characters".to_string()));
    }
    if req.password.len() < 6 {
        return Err((StatusCode::BAD_REQUEST, "password must be at least 6 characters".to_string()));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(internal)?
        .to_string();

    let user_id = db::create_user(&state.db, &email, &username, &hash)
        .await
        .map_err(|e| {
            let unique_violation = e
                .as_database_error()
                .and_then(|d| d.code())
                .is_some_and(|c| c == "23505");
            if unique_violation {
                (StatusCode::CONFLICT, "email already registered".to_string())
            } else {
                internal(e)
            }
        })?;

    let token = issue_jwt(&state.jwt_secret, user_id, &username, &email).map_err(internal)?;
    Ok(Json(AuthResponse { token, user_id, username, email }))
}

pub async fn login(State(state): State<Arc<AppState>>, Json(req): Json<LoginRequest>) -> Result<Json<AuthResponse>, ApiError> {
    let email = req.email.trim().to_ascii_lowercase();
    let user = db::find_user_by_email(&state.db, &email)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;

    let parsed = PasswordHash::new(&user.password_hash).map_err(internal)?;
    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;

    let token = issue_jwt(&state.jwt_secret, user.id, &user.username, &user.email).map_err(internal)?;
    Ok(Json(AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
    }))
}

pub async fn list_channels(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Result<Json<Vec<ChannelInfo>>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let channels = db::list_channels_for_user(&state.db, claims.sub).await.map_err(internal)?;
    Ok(Json(channels))
}

/// Returns `Ok(true)` if `user_id` may read/write `channel_id`.
pub async fn can_access_channel(state: &AppState, user_id: i64, channel_id: i64) -> Result<bool, sqlx::Error> {
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
    if !can_access_channel(&state, claims.sub, channel_id).await.map_err(internal)? {
        return Err((StatusCode::FORBIDDEN, "no access to channel".to_string()));
    }
    let messages = db::list_messages(&state.db, channel_id, 100).await.map_err(internal)?;
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
        return Err((StatusCode::BAD_REQUEST, "cannot DM yourself".to_string()));
    }
    let peer_name = db::find_username(&state.db, peer_id)
        .await
        .map_err(internal)?
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?;
    let id = db::get_or_create_dm(&state.db, claims.sub, peer_id).await.map_err(internal)?;
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
