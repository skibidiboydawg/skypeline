use skypeline_shared::{ChannelInfo, MessageDto};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::str::FromStr;

/// Connects to Aiven PostgreSQL (honoring `?sslmode=require` via rustls TLS)
/// and runs idempotent table migrations.
pub async fn connect_and_migrate(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let opts = PgConnectOptions::from_str(database_url)?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(opts)
        .await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id            BIGSERIAL PRIMARY KEY,
            email         VARCHAR(255) NOT NULL UNIQUE,
            username      VARCHAR(64)  NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS channels (
            id         BIGSERIAL PRIMARY KEY,
            name       VARCHAR(128) NOT NULL UNIQUE,
            kind       VARCHAR(16)  NOT NULL DEFAULT 'text',
            created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id         BIGSERIAL PRIMARY KEY,
            channel_id BIGINT      NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
            user_id    BIGINT      NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
            username   VARCHAR(64) NOT NULL,
            content    TEXT        NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages (channel_id, id)",
    )
    .execute(pool)
    .await?;

    for (name, kind) in [
        ("general", "text"),
        ("random", "text"),
        ("General Voice", "voice"),
    ] {
        sqlx::query("INSERT INTO channels (name, kind) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING")
            .bind(name)
            .bind(kind)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub password_hash: String,
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    username: &str,
    password_hash: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO users (email, username, password_hash) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(email)
    .bind(username)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("SELECT id, email, username, password_hash FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| UserRow {
        id: r.get("id"),
        email: r.get("email"),
        username: r.get("username"),
        password_hash: r.get("password_hash"),
    }))
}

pub async fn find_username(pool: &PgPool, user_id: i64) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get("username")))
}

pub struct ChannelRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
}

pub async fn get_channel(pool: &PgPool, channel_id: i64) -> Result<Option<ChannelRow>, sqlx::Error> {
    let row = sqlx::query("SELECT id, name, kind FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| ChannelRow {
        id: r.get("id"),
        name: r.get("name"),
        kind: r.get("kind"),
    }))
}

/// DM channels are named `dm:<low_id>:<high_id>`.
pub fn dm_channel_name(a: i64, b: i64) -> String {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    format!("dm:{lo}:{hi}")
}

pub fn dm_members(name: &str) -> Option<(i64, i64)> {
    let rest = name.strip_prefix("dm:")?;
    let (a, b) = rest.split_once(':')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

pub async fn get_or_create_dm(pool: &PgPool, a: i64, b: i64) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO channels (name, kind) VALUES ($1, 'dm')
         ON CONFLICT (name) DO UPDATE SET kind = EXCLUDED.kind
         RETURNING id",
    )
    .bind(dm_channel_name(a, b))
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

/// Lists public text/voice channels plus only the DM channels this user belongs to.
/// DM channel names are replaced with the peer's username for display.
pub async fn list_channels_for_user(pool: &PgPool, user_id: i64) -> Result<Vec<ChannelInfo>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, name, kind FROM channels ORDER BY id ASC")
        .fetch_all(pool)
        .await?;

    let mut out = Vec::new();
    let mut dm_peers: Vec<(usize, i64)> = Vec::new();
    for r in rows {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let kind: String = r.get("kind");
        if kind == "dm" {
            let Some((lo, hi)) = dm_members(&name) else { continue };
            if lo != user_id && hi != user_id {
                continue;
            }
            let peer = if lo == user_id { hi } else { lo };
            dm_peers.push((out.len(), peer));
            out.push(ChannelInfo { id, name, kind, peer_user_id: Some(peer) });
        } else {
            out.push(ChannelInfo { id, name, kind, peer_user_id: None });
        }
    }

    if !dm_peers.is_empty() {
        let ids: Vec<i64> = dm_peers.iter().map(|(_, p)| *p).collect();
        let rows = sqlx::query("SELECT id, username FROM users WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(pool)
            .await?;
        let names: HashMap<i64, String> = rows
            .into_iter()
            .map(|r| (r.get::<i64, _>("id"), r.get::<String, _>("username")))
            .collect();
        for (idx, peer) in dm_peers {
            if let Some(n) = names.get(&peer) {
                out[idx].name = n.clone();
            }
        }
    }
    Ok(out)
}

pub async fn insert_message(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
    username: &str,
    content: &str,
) -> Result<MessageDto, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO messages (channel_id, user_id, username, content)
         VALUES ($1, $2, $3, $4)
         RETURNING id, EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at",
    )
    .bind(channel_id)
    .bind(user_id)
    .bind(username)
    .bind(content)
    .fetch_one(pool)
    .await?;

    Ok(MessageDto {
        id: row.get("id"),
        channel_id,
        user_id,
        username: username.to_string(),
        content: content.to_string(),
        created_at: row.get("created_at"),
    })
}

pub async fn list_messages(pool: &PgPool, channel_id: i64, limit: i64) -> Result<Vec<MessageDto>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, channel_id, user_id, username, content,
                EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at
         FROM messages
         WHERE channel_id = $1
         ORDER BY id DESC
         LIMIT $2",
    )
    .bind(channel_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut msgs: Vec<MessageDto> = rows
        .into_iter()
        .map(|r| MessageDto {
            id: r.get("id"),
            channel_id: r.get("channel_id"),
            user_id: r.get("user_id"),
            username: r.get("username"),
            content: r.get("content"),
            created_at: r.get("created_at"),
        })
        .collect();
    msgs.reverse();
    Ok(msgs)
}
