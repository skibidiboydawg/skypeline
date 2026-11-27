use skypeline_shared::{ChannelInfo, ChannelMemberInfo, MessageDto};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};
use std::collections::HashSet;
use std::str::FromStr;

/// Connects to Neon/Lakebase Postgres (honoring `?sslmode=require` via rustls
/// TLS) and runs idempotent table migrations.
///
/// `database_url` should be the pooled URL used for normal app traffic.
/// `migration_url` should be Neon's direct (non-pooled) URL; when absent we
/// fall back to the pooled URL for compatibility with hosts that inject only
/// `DATABASE_URL`.
pub async fn connect_and_migrate(
    database_url: &str,
    migration_url: Option<&str>,
) -> Result<PgPool, sqlx::Error> {
    let opts = PgConnectOptions::from_str(database_url)?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(opts)
        .await?;

    let ddl_url = migration_url
        .filter(|url| !url.is_empty())
        .unwrap_or(database_url);
    let ddl_opts = PgConnectOptions::from_str(ddl_url)?;
    let migration_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(ddl_opts)
        .await?;
    run_migrations(&migration_pool).await?;
    migration_pool.close().await;

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

    // Neon Auth owns the user identity. This links the stable Neon user UUID
    // to Skypeline's integer primary keys (used by messages and DM channels).
    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS neon_auth_user_id TEXT")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_neon_auth_user_id
         ON users (neon_auth_user_id) WHERE neon_auth_user_id IS NOT NULL",
    )
    .execute(pool)
    .await?;

    // Avatars are stored as normalized 128x128 JPEG bytes in the users row.
    for column in [
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_data BYTEA",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_mime TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_hash TEXT",
    ] {
        sqlx::query(column).execute(pool).await?;
    }

    // Friendships are directional rows; accepting inserts the reciprocal row so
    // "accepted" is always symmetric and cheap to look up.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS friendships (
            id         BIGSERIAL PRIMARY KEY,
            user_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            friend_id  BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            status     VARCHAR(16) NOT NULL CHECK (status IN ('pending', 'accepted')),
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE (user_id, friend_id),
            CHECK (user_id <> friend_id)
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_friendships_user_id ON friendships (user_id, status)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_friendships_friend_id ON friendships (friend_id, status)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS channel_members (
            channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
            user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            joined_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (channel_id, user_id)
        )
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_channel_members_user_id ON channel_members (user_id, channel_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages (channel_id, id)")
        .execute(pool)
        .await?;

    sqlx::query("ALTER TABLE messages ADD COLUMN IF NOT EXISTS edited_at TIMESTAMPTZ")
        .execute(pool)
        .await?;

    // Allow multiple group chats to share a display name while keeping DM keys unique.
    sqlx::query("ALTER TABLE channels DROP CONSTRAINT IF EXISTS channels_name_key")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_channels_dm_name_unique
         ON channels (name) WHERE kind = 'dm'",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub username: String,
}

pub async fn find_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("SELECT id, email, username FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| UserRow {
        id: r.get("id"),
        email: r.get("email"),
        username: r.get("username"),
    }))
}

/// Placeholder stored in the legacy column now that passwords live in Neon Auth.
const NEON_PASSWORD_PLACEHOLDER: &str = "managed-by-neon-auth";

pub async fn find_user_by_neon_id(
    pool: &PgPool,
    neon_auth_user_id: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query("SELECT id, email, username FROM users WHERE neon_auth_user_id = $1")
        .bind(neon_auth_user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| UserRow {
        id: r.get("id"),
        email: r.get("email"),
        username: r.get("username"),
    }))
}

/// Creates the local projection of a Neon Auth user. Returns the existing row
/// when the same Neon identity was already provisioned (e.g. a retried request).
pub async fn create_neon_user(
    pool: &PgPool,
    neon_auth_user_id: &str,
    email: &str,
    username: &str,
) -> Result<UserRow, sqlx::Error> {
    if let Some(user) = find_user_by_neon_id(pool, neon_auth_user_id).await? {
        if user.email.eq_ignore_ascii_case(email) {
            return Ok(user);
        }
    }

    let row = sqlx::query(
        "INSERT INTO users (email, username, password_hash, neon_auth_user_id)
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, username",
    )
    .bind(email)
    .bind(username)
    .bind(NEON_PASSWORD_PLACEHOLDER)
    .bind(neon_auth_user_id)
    .fetch_one(pool)
    .await?;

    Ok(UserRow {
        id: row.get("id"),
        email: row.get("email"),
        username: row.get("username"),
    })
}

/// Links a verified Neon Auth identity to a legacy row with the same email.
pub async fn link_neon_user(
    pool: &PgPool,
    user_id: i64,
    neon_auth_user_id: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query(
        "UPDATE users SET neon_auth_user_id = $1
         WHERE id = $2 AND (neon_auth_user_id IS NULL OR neon_auth_user_id = $1)
         RETURNING id, email, username",
    )
    .bind(neon_auth_user_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| UserRow {
        id: r.get("id"),
        email: r.get("email"),
        username: r.get("username"),
    }))
}

pub async fn find_username(pool: &PgPool, user_id: i64) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.get("username")))
}

pub async fn avatar_hash_for_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT avatar_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| r.get("avatar_hash")))
}

/// Accepted friend ids in both directions (the reciprocal row is stored too).
pub async fn accepted_friend_ids(pool: &PgPool, user_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT CASE WHEN user_id = $1 THEN friend_id ELSE user_id END AS other_id
         FROM friendships
         WHERE status = 'accepted' AND (user_id = $1 OR friend_id = $1)",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.get("other_id")).collect())
}

pub async fn are_friends(pool: &PgPool, a: i64, b: i64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "SELECT EXISTS (
            SELECT 1 FROM friendships
            WHERE status = 'accepted'
              AND ((user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1))
         ) AS friends",
    )
    .bind(a)
    .bind(b)
    .fetch_one(pool)
    .await?;
    Ok(row.get("friends"))
}

pub struct ChannelRow {
    pub id: i64,
    pub name: String,
    pub kind: String,
}

pub async fn get_channel(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Option<ChannelRow>, sqlx::Error> {
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
    let id: i64 = row.get("id");
    for uid in [a, b] {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id) VALUES ($1, $2)
             ON CONFLICT (channel_id, user_id) DO NOTHING",
        )
        .bind(id)
        .bind(uid)
        .execute(pool)
        .await?;
    }
    Ok(id)
}

pub async fn is_channel_member(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "SELECT EXISTS(
            SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2
         ) AS is_member",
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.get("is_member"))
}

pub async fn channel_member_ids(pool: &PgPool, channel_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query("SELECT user_id FROM channel_members WHERE channel_id = $1")
        .bind(channel_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.get("user_id")).collect())
}

pub async fn channel_members(
    pool: &PgPool,
    channel_id: i64,
) -> Result<Vec<ChannelMemberInfo>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT u.id, u.username, u.avatar_hash
         FROM channel_members m
         JOIN users u ON u.id = m.user_id
         WHERE m.channel_id = $1
         ORDER BY u.username ASC",
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ChannelMemberInfo {
            user_id: r.get("id"),
            username: r.get("username"),
            avatar_hash: r.get("avatar_hash"),
        })
        .collect())
}

pub async fn create_group(
    pool: &PgPool,
    creator_id: i64,
    name: &str,
    member_ids: &[i64],
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("INSERT INTO channels (name, kind) VALUES ($1, 'group') RETURNING id")
        .bind(name)
        .fetch_one(pool)
        .await?;
    let id: i64 = row.get("id");
    let mut ids: Vec<i64> = member_ids.to_vec();
    if !ids.contains(&creator_id) {
        ids.push(creator_id);
    }
    ids.sort_unstable();
    ids.dedup();
    for uid in ids {
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id) VALUES ($1, $2)
             ON CONFLICT (channel_id, user_id) DO NOTHING",
        )
        .bind(id)
        .bind(uid)
        .execute(pool)
        .await?;
    }
    Ok(id)
}

pub async fn add_group_member(
    pool: &PgPool,
    channel_id: i64,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id) VALUES ($1, $2)
         ON CONFLICT (channel_id, user_id) DO NOTHING",
    )
    .bind(channel_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Lists DM and group channels visible to this user, batching member and peer
/// username lookups to avoid N+1 database round-trips.
pub async fn list_channels_for_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<ChannelInfo>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.kind
         FROM channels c
         WHERE c.kind = 'group' AND EXISTS (
             SELECT 1 FROM channel_members m WHERE m.channel_id = c.id AND m.user_id = $1
         )
         OR c.kind = 'dm' AND (
             c.name = $2 OR c.name LIKE $3 OR c.name LIKE $4
         )
         ORDER BY c.id ASC",
    )
    .bind(user_id)
    .bind(dm_channel_name(user_id, user_id))
    .bind(format!("dm:{user_id}:%"))
    .bind(format!("dm:%:{user_id}"))
    .fetch_all(pool)
    .await?;

    let friends: HashSet<i64> = accepted_friend_ids(pool, user_id)
        .await?
        .into_iter()
        .collect();

    struct PendingChannel {
        id: i64,
        name: String,
        kind: String,
        peer_user_id: Option<i64>,
    }

    let mut pending = Vec::new();
    let mut channel_ids = Vec::new();
    let mut peer_ids = Vec::new();

    for r in rows {
        let id: i64 = r.get("id");
        let name: String = r.get("name");
        let kind: String = r.get("kind");
        if kind == "dm" {
            let Some((lo, hi)) = dm_members(&name) else {
                continue;
            };
            if lo != user_id && hi != user_id {
                continue;
            }
            let peer = if lo == user_id { hi } else { lo };
            if !friends.contains(&peer) {
                continue;
            }
            peer_ids.push(peer);
            channel_ids.push(id);
            pending.push(PendingChannel {
                id,
                name,
                kind,
                peer_user_id: Some(peer),
            });
        } else if kind == "group" {
            channel_ids.push(id);
            pending.push(PendingChannel {
                id,
                name,
                kind,
                peer_user_id: None,
            });
        }
    }

    if pending.is_empty() {
        return Ok(Vec::new());
    }

    let member_rows = sqlx::query(
        "SELECT m.channel_id, u.id AS user_id, u.username, u.avatar_hash
         FROM channel_members m
         JOIN users u ON u.id = m.user_id
         WHERE m.channel_id = ANY($1)
         ORDER BY lower(u.username) ASC",
    )
    .bind(&channel_ids)
    .fetch_all(pool)
    .await?;

    let mut members_by_channel: std::collections::HashMap<i64, Vec<ChannelMemberInfo>> =
        std::collections::HashMap::new();
    for r in member_rows {
        let cid: i64 = r.get("channel_id");
        members_by_channel
            .entry(cid)
            .or_default()
            .push(ChannelMemberInfo {
                user_id: r.get("user_id"),
                username: r.get("username"),
                avatar_hash: r.get("avatar_hash"),
            });
    }

    let mut peer_names: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    if !peer_ids.is_empty() {
        peer_ids.sort_unstable();
        peer_ids.dedup();
        let peer_rows = sqlx::query("SELECT id, username FROM users WHERE id = ANY($1)")
            .bind(&peer_ids)
            .fetch_all(pool)
            .await?;
        for r in peer_rows {
            peer_names.insert(r.get("id"), r.get("username"));
        }
    }

    let out = pending
        .into_iter()
        .map(|p| {
            let name = if let Some(peer) = p.peer_user_id {
                peer_names
                    .get(&peer)
                    .cloned()
                    .unwrap_or_else(|| format!("User {peer}"))
            } else {
                p.name
            };
            let members = members_by_channel.remove(&p.id).unwrap_or_default();
            ChannelInfo {
                id: p.id,
                name,
                kind: p.kind,
                peer_user_id: p.peer_user_id,
                members,
            }
        })
        .collect();
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
        edited_at: None,
    })
}

pub async fn list_messages(
    pool: &PgPool,
    channel_id: i64,
    before_id: Option<i64>,
    limit: i64,
) -> Result<Vec<MessageDto>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, channel_id, user_id, username, content,
                EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at,
                EXTRACT(EPOCH FROM edited_at)::BIGINT AS edited_at
         FROM messages
         WHERE channel_id = $1
           AND ($2::BIGINT IS NULL OR id < $2)
         ORDER BY id DESC
         LIMIT $3",
    )
    .bind(channel_id)
    .bind(before_id)
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
            edited_at: r.get("edited_at"),
        })
        .collect();
    msgs.reverse();
    Ok(msgs)
}

/// Edits a message owned by `user_id`, returning `(channel_id, edited_at)` on success.
pub async fn edit_message(
    pool: &PgPool,
    message_id: i64,
    user_id: i64,
    content: &str,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let row = sqlx::query(
        "UPDATE messages
         SET content = $3, edited_at = NOW()
         WHERE id = $1 AND user_id = $2
         RETURNING channel_id, EXTRACT(EPOCH FROM edited_at)::BIGINT AS edited_at",
    )
    .bind(message_id)
    .bind(user_id)
    .bind(content)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| (r.get("channel_id"), r.get("edited_at"))))
}

/// Deletes a message owned by `user_id`, returning `channel_id` on success.
pub async fn delete_message(
    pool: &PgPool,
    message_id: i64,
    user_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query(
        "DELETE FROM messages
         WHERE id = $1 AND user_id = $2
         RETURNING channel_id",
    )
    .bind(message_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get("channel_id")))
}
