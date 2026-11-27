use crate::auth::{bearer, internal, ApiError};
use crate::{db, AppState};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use skypeline_shared::{
    FriendInfo, FriendRequest, FriendRespondRequest, FriendshipStatus, PresenceStatus,
    StatusResponse, WsServerMsg,
};
use sqlx::Row;
use std::collections::HashSet;
use std::sync::Arc;

/// `POST /api/friends/request` — find a user by exact email or exact (case
/// insensitive) username and create a pending friendship request.
pub async fn request(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<FriendRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let query = req.query.trim();
    if query.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "enter a username or email".to_string(),
        ));
    }

    let row = if query.contains('@') {
        sqlx::query("SELECT id FROM users WHERE lower(email) = $1")
            .bind(query.to_ascii_lowercase())
            .fetch_optional(&state.db)
            .await
            .map_err(internal)?
    } else {
        sqlx::query("SELECT id FROM users WHERE lower(username) = lower($1)")
            .bind(query)
            .fetch_optional(&state.db)
            .await
            .map_err(internal)?
    };
    let target_id: i64 = row
        .ok_or((StatusCode::NOT_FOUND, "user not found".to_string()))?
        .get("id");
    if target_id == claims.sub {
        return Err((
            StatusCode::BAD_REQUEST,
            "cannot friend yourself".to_string(),
        ));
    }

    let existing = sqlx::query(
        "SELECT user_id, status FROM friendships
         WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)",
    )
    .bind(claims.sub)
    .bind(target_id)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;
    for row in &existing {
        let owner_id: i64 = row.get("user_id");
        let status: String = row.get("status");
        if status == "accepted" {
            return Err((StatusCode::CONFLICT, "you are already friends".to_string()));
        }
        if status == "pending" {
            return if owner_id == claims.sub {
                Ok(Json(StatusResponse {
                    ok: true,
                    message: "Friend request already sent.".to_string(),
                }))
            } else {
                Err((
                    StatusCode::CONFLICT,
                    "this user already sent you a friend request; accept it from requests"
                        .to_string(),
                ))
            };
        }
    }

    sqlx::query(
        "INSERT INTO friendships (user_id, friend_id, status) VALUES ($1, $2, 'pending')
         ON CONFLICT (user_id, friend_id) DO NOTHING",
    )
    .bind(claims.sub)
    .bind(target_id)
    .execute(&state.db)
    .await
    .map_err(internal)?;

    state.hub.send(
        crate::hub::Scope::Users(vec![target_id]),
        None,
        &WsServerMsg::FriendsUpdated,
    );

    Ok(Json(StatusResponse {
        ok: true,
        message: "Friend request sent.".to_string(),
    }))
}

/// `POST /api/friends/respond` — accept or decline an incoming pending request.
pub async fn respond(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<FriendRespondRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    if req.requester_id == claims.sub {
        return Err((
            StatusCode::BAD_REQUEST,
            "cannot respond to yourself".to_string(),
        ));
    }

    let row = sqlx::query("SELECT status FROM friendships WHERE user_id = $1 AND friend_id = $2")
        .bind(req.requester_id)
        .bind(claims.sub)
        .fetch_optional(&state.db)
        .await
        .map_err(internal)?;
    let status: String = row
        .ok_or((
            StatusCode::NOT_FOUND,
            "friend request not found".to_string(),
        ))?
        .get("status");
    if status != "pending" {
        return Err((
            StatusCode::NOT_FOUND,
            "friend request not found".to_string(),
        ));
    }

    if req.accept {
        sqlx::query("UPDATE friendships SET status = 'accepted', updated_at = NOW() WHERE user_id = $1 AND friend_id = $2")
            .bind(req.requester_id)
            .bind(claims.sub)
            .execute(&state.db)
            .await
            .map_err(internal)?;
        sqlx::query(
            "INSERT INTO friendships (user_id, friend_id, status) VALUES ($1, $2, 'accepted')
             ON CONFLICT (user_id, friend_id) DO UPDATE
             SET status = 'accepted', updated_at = NOW()",
        )
        .bind(claims.sub)
        .bind(req.requester_id)
        .execute(&state.db)
        .await
        .map_err(internal)?;

        for user_id in [claims.sub, req.requester_id] {
            let ids = db::accepted_friend_ids(&state.db, user_id)
                .await
                .map_err(internal)?;
            state.hub.refresh_friends(user_id, ids);
        }
        state.hub.broadcast_presence();
        state.hub.send(
            crate::hub::Scope::Users(vec![req.requester_id]),
            None,
            &WsServerMsg::FriendsUpdated,
        );
        state.hub.send(
            crate::hub::Scope::Users(vec![req.requester_id]),
            None,
            &WsServerMsg::ChannelsUpdated,
        );
        Ok(Json(StatusResponse {
            ok: true,
            message: "Friend request accepted.".to_string(),
        }))
    } else {
        sqlx::query(
            "DELETE FROM friendships
             WHERE user_id = $1 AND friend_id = $2 AND status = 'pending'",
        )
        .bind(req.requester_id)
        .bind(claims.sub)
        .execute(&state.db)
        .await
        .map_err(internal)?;
        Ok(Json(StatusResponse {
            ok: true,
            message: "Friend request declined.".to_string(),
        }))
    }
}

fn direction_rank(info: &FriendInfo) -> u8 {
    match info.direction.as_str() {
        "mutual" => 0,
        "incoming" => 1,
        _ => 2,
    }
}

/// `GET /api/friends` — only rows from `friendships` involving the caller.
pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<FriendInfo>>, ApiError> {
    let claims = bearer(&headers, &state.jwt_secret)?;
    let rows = sqlx::query(
        "SELECT f.user_id AS owner_id,
                CASE WHEN f.user_id = $1 THEN f.friend_id ELSE f.user_id END AS other_id,
                f.status AS status,
                u.username AS username,
                u.avatar_hash AS avatar_hash
         FROM friendships f
         JOIN users u ON u.id = CASE WHEN f.user_id = $1 THEN f.friend_id ELSE f.user_id END
         WHERE f.user_id = $1 OR f.friend_id = $1",
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let mut friends: Vec<FriendInfo> = rows
        .into_iter()
        .map(|row| {
            let owner_id: i64 = row.get("owner_id");
            let other_id: i64 = row.get("other_id");
            let status: String = row.get("status");
            let (status, direction) = match status.as_str() {
                "accepted" => (FriendshipStatus::Accepted, "mutual"),
                _ if owner_id == claims.sub => (FriendshipStatus::Pending, "outgoing"),
                _ => (FriendshipStatus::Pending, "incoming"),
            };
            let presence = state
                .hub
                .online_users
                .get(&other_id)
                .map(|entry| entry.value().status)
                .unwrap_or(PresenceStatus::Offline);
            FriendInfo {
                user_id: other_id,
                username: row.get("username"),
                status,
                direction: direction.to_string(),
                online: state.hub.online_users.contains_key(&other_id),
                presence,
                avatar_hash: row.get("avatar_hash"),
            }
        })
        .collect();
    friends.sort_by(|a, b| {
        direction_rank(a)
            .cmp(&direction_rank(b))
            .then_with(|| {
                a.username
                    .to_ascii_lowercase()
                    .cmp(&b.username.to_ascii_lowercase())
            })
            .then_with(|| a.user_id.cmp(&b.user_id))
    });
    // Accepting stores two symmetric accepted rows; the caller only needs one
    // card per person. Sorted accepted rows come first, so keep the first.
    let mut seen = HashSet::new();
    friends.retain(|friend| seen.insert(friend.user_id));
    Ok(Json(friends))
}
