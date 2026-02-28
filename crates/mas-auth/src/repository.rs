//! Database repository for auth entities (users, orgs, memberships, etc.)

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::models::*;
use crate::AuthError;

/// Parse a datetime string from SQLite into DateTime<Utc>
fn parse_dt(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

// ─── Users ──────────────────────────────────────────────

/// Create a new user
pub async fn create_user(
    pool: &SqlitePool,
    id: &str,
    email: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<User, AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint") {
            AuthError::EmailTaken(email.to_string())
        } else {
            AuthError::Database(e.to_string())
        }
    })?;

    Ok(User {
        id: id.to_string(),
        email: email.to_string(),
        display_name: display_name.to_string(),
        password_hash: password_hash.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

/// Find a user by email
pub async fn find_user_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, AuthError> {
    let row = sqlx::query("SELECT id, email, display_name, password_hash, created_at, updated_at FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(row.map(|r| user_from_row(&r)))
}

/// Find a user by ID
pub async fn find_user_by_id(pool: &SqlitePool, id: &str) -> Result<Option<User>, AuthError> {
    let row = sqlx::query("SELECT id, email, display_name, password_hash, created_at, updated_at FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(row.map(|r| user_from_row(&r)))
}

/// Update user profile
pub async fn update_user(
    pool: &SqlitePool,
    id: &str,
    display_name: Option<&str>,
    password_hash: Option<&str>,
) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if let Some(name) = display_name {
        sqlx::query("UPDATE users SET display_name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Database(e.to_string()))?;
    }

    if let Some(hash) = password_hash {
        sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(hash)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Database(e.to_string()))?;
    }

    Ok(())
}

fn user_from_row(row: &SqliteRow) -> User {
    User {
        id: row.get("id"),
        email: row.get("email"),
        display_name: row.get("display_name"),
        password_hash: row.get("password_hash"),
        created_at: parse_dt(row.get("created_at")),
        updated_at: parse_dt(row.get("updated_at")),
    }
}

// ─── Organizations ──────────────────────────────────────

/// Create a new organization
pub async fn create_org(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    slug: &str,
    parent_id: Option<&str>,
) -> Result<Organization, AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO organizations (id, name, slug, parent_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(name)
    .bind(slug)
    .bind(parent_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint") {
            AuthError::OrgSlugTaken(slug.to_string())
        } else {
            AuthError::Database(e.to_string())
        }
    })?;

    Ok(Organization {
        id: id.to_string(),
        name: name.to_string(),
        slug: slug.to_string(),
        parent_id: parent_id.map(|s| s.to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

/// Find an organization by ID
pub async fn find_org_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Organization>, AuthError> {
    let row = sqlx::query("SELECT id, name, slug, parent_id, created_at, updated_at FROM organizations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(row.map(|r| org_from_row(&r)))
}

/// Update an organization
pub async fn update_org(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    slug: Option<&str>,
) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if let Some(name) = name {
        sqlx::query("UPDATE organizations SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AuthError::Database(e.to_string()))?;
    }

    if let Some(slug) = slug {
        sqlx::query("UPDATE organizations SET slug = ?, updated_at = ? WHERE id = ?")
            .bind(slug)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint") {
                    AuthError::OrgSlugTaken(slug.to_string())
                } else {
                    AuthError::Database(e.to_string())
                }
            })?;
    }

    Ok(())
}

/// Delete an organization
pub async fn delete_org(pool: &SqlitePool, id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM organizations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// List child organizations of a given parent
pub async fn list_child_orgs(pool: &SqlitePool, parent_id: &str) -> Result<Vec<Organization>, AuthError> {
    let rows = sqlx::query("SELECT id, name, slug, parent_id, created_at, updated_at FROM organizations WHERE parent_id = ?")
        .bind(parent_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows.iter().map(org_from_row).collect())
}

fn org_from_row(row: &SqliteRow) -> Organization {
    Organization {
        id: row.get("id"),
        name: row.get("name"),
        slug: row.get("slug"),
        parent_id: row.get("parent_id"),
        created_at: parse_dt(row.get("created_at")),
        updated_at: parse_dt(row.get("updated_at")),
    }
}

// ─── Org Memberships ────────────────────────────────────

/// Add a user to an organization
pub async fn add_membership(
    pool: &SqlitePool,
    user_id: &str,
    org_id: &str,
    role: OrgRole,
) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query("INSERT INTO org_memberships (user_id, org_id, role, created_at) VALUES (?, ?, ?, ?)")
        .bind(user_id)
        .bind(org_id)
        .bind(role.as_str())
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(())
}

/// Remove a user from an organization
pub async fn remove_membership(pool: &SqlitePool, user_id: &str, org_id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM org_memberships WHERE user_id = ? AND org_id = ?")
        .bind(user_id)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Update a member's role
pub async fn update_membership_role(
    pool: &SqlitePool,
    user_id: &str,
    org_id: &str,
    role: OrgRole,
) -> Result<(), AuthError> {
    sqlx::query("UPDATE org_memberships SET role = ? WHERE user_id = ? AND org_id = ?")
        .bind(role.as_str())
        .bind(user_id)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Get a user's role in an organization
pub async fn get_membership(
    pool: &SqlitePool,
    user_id: &str,
    org_id: &str,
) -> Result<Option<OrgRole>, AuthError> {
    let row = sqlx::query("SELECT role FROM org_memberships WHERE user_id = ? AND org_id = ?")
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(row.and_then(|r| {
        let role_str: String = r.get("role");
        OrgRole::from_str(&role_str)
    }))
}

/// List all organizations a user belongs to (with their role)
pub async fn list_user_orgs(pool: &SqlitePool, user_id: &str) -> Result<Vec<OrgWithRole>, AuthError> {
    let rows = sqlx::query(
        "SELECT o.id, o.name, o.slug, o.parent_id, o.created_at, o.updated_at, m.role
         FROM organizations o
         JOIN org_memberships m ON o.id = m.org_id
         WHERE m.user_id = ?"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let role_str: String = r.get("role");
            OrgWithRole {
                org: org_from_row(r),
                role: OrgRole::from_str(&role_str).unwrap_or(OrgRole::Member),
            }
        })
        .collect())
}

/// List all members of an organization
pub async fn list_org_members(pool: &SqlitePool, org_id: &str) -> Result<Vec<MemberInfo>, AuthError> {
    let rows = sqlx::query(
        "SELECT u.id as user_id, u.email, u.display_name, m.role, m.created_at
         FROM users u
         JOIN org_memberships m ON u.id = m.user_id
         WHERE m.org_id = ?"
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| {
            let role_str: String = r.get("role");
            MemberInfo {
                user_id: r.get("user_id"),
                email: r.get("email"),
                display_name: r.get("display_name"),
                role: OrgRole::from_str(&role_str).unwrap_or(OrgRole::Member),
                joined_at: parse_dt(r.get("created_at")),
            }
        })
        .collect())
}

// ─── System-Org Associations ────────────────────────────

/// Associate a system with an organization
pub async fn add_system_org(pool: &SqlitePool, system_name: &str, org_id: &str) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query("INSERT OR IGNORE INTO system_orgs (system_name, org_id, created_at) VALUES (?, ?, ?)")
        .bind(system_name)
        .bind(org_id)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Remove a system-org association
pub async fn remove_system_org(pool: &SqlitePool, system_name: &str, org_id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM system_orgs WHERE system_name = ? AND org_id = ?")
        .bind(system_name)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// List systems in an organization
pub async fn list_org_systems(pool: &SqlitePool, org_id: &str) -> Result<Vec<String>, AuthError> {
    let rows = sqlx::query("SELECT system_name FROM system_orgs WHERE org_id = ?")
        .bind(org_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get("system_name")).collect())
}

/// List organizations a system belongs to
pub async fn list_system_orgs(pool: &SqlitePool, system_name: &str) -> Result<Vec<String>, AuthError> {
    let rows = sqlx::query("SELECT org_id FROM system_orgs WHERE system_name = ?")
        .bind(system_name)
        .fetch_all(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get("org_id")).collect())
}

/// Check if a user has access to a system (via org membership OR direct ownership)
pub async fn user_has_system_access(
    pool: &SqlitePool,
    user_id: &str,
    system_name: &str,
) -> Result<bool, AuthError> {
    let row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM (
            SELECT 1 FROM system_orgs so
            JOIN org_memberships m ON so.org_id = m.org_id
            WHERE m.user_id = ? AND so.system_name = ?
            UNION
            SELECT 1 FROM system_owners
            WHERE user_id = ? AND system_name = ?
        )"
    )
    .bind(user_id)
    .bind(system_name)
    .bind(user_id)
    .bind(system_name)
    .fetch_one(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    let count: i64 = row.get("cnt");
    Ok(count > 0)
}

/// List all system names a user has access to (via org memberships OR direct ownership)
pub async fn list_user_systems(pool: &SqlitePool, user_id: &str) -> Result<Vec<String>, AuthError> {
    let rows = sqlx::query(
        "SELECT DISTINCT system_name FROM (
            SELECT so.system_name
            FROM system_orgs so
            JOIN org_memberships m ON so.org_id = m.org_id
            WHERE m.user_id = ?
            UNION
            SELECT system_name
            FROM system_owners
            WHERE user_id = ?
        )"
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get("system_name")).collect())
}

// ─── System Ownership ──────────────────────────────────

/// Record that a user owns a system
pub async fn add_system_owner(
    pool: &SqlitePool,
    system_name: &str,
    user_id: &str,
) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT OR IGNORE INTO system_owners (system_name, user_id, created_at) VALUES (?, ?, ?)",
    )
    .bind(system_name)
    .bind(user_id)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Remove system ownership record
pub async fn remove_system_owner(
    pool: &SqlitePool,
    system_name: &str,
    user_id: &str,
) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM system_owners WHERE system_name = ? AND user_id = ?")
        .bind(system_name)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Remove all ownership records for a system
pub async fn delete_system_owners(pool: &SqlitePool, system_name: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM system_owners WHERE system_name = ?")
        .bind(system_name)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

// ─── Sessions ───────────────────────────────────────────

/// Record a session ownership
pub async fn create_session_record(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    system_name: &str,
) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query("INSERT INTO sessions (id, user_id, system_name, created_at) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(user_id)
        .bind(system_name)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Check if a user owns a session
pub async fn user_owns_session(pool: &SqlitePool, user_id: &str, session_id: &str) -> Result<bool, AuthError> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM sessions WHERE id = ? AND user_id = ?")
        .bind(session_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    let count: i64 = row.get("cnt");
    Ok(count > 0)
}

/// List session IDs owned by a user
pub async fn list_user_sessions(pool: &SqlitePool, user_id: &str) -> Result<Vec<String>, AuthError> {
    let rows = sqlx::query("SELECT id FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get("id")).collect())
}

/// Delete a session record
pub async fn delete_session_record(pool: &SqlitePool, session_id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

// ─── Refresh Tokens ─────────────────────────────────────

/// Store a refresh token hash
pub async fn store_refresh_token(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    token_hash: &str,
    expires_at: &str,
) -> Result<(), AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Find a refresh token by its hash and verify it hasn't expired
pub async fn find_valid_refresh_token(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<(String, String)>, AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let row = sqlx::query(
        "SELECT id, user_id FROM refresh_tokens WHERE token_hash = ? AND expires_at > ?"
    )
    .bind(token_hash)
    .bind(&now)
    .fetch_optional(pool)
    .await
    .map_err(|e| AuthError::Database(e.to_string()))?;

    Ok(row.map(|r| {
        let id: String = r.get("id");
        let user_id: String = r.get("user_id");
        (id, user_id)
    }))
}

/// Delete a specific refresh token
pub async fn delete_refresh_token(pool: &SqlitePool, id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM refresh_tokens WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Delete all refresh tokens for a user
pub async fn delete_user_refresh_tokens(pool: &SqlitePool, user_id: &str) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(())
}

/// Clean up expired refresh tokens
pub async fn cleanup_expired_tokens(pool: &SqlitePool) -> Result<u64, AuthError> {
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at <= ?")
        .bind(&now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;
    Ok(result.rows_affected())
}
