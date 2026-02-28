//! Domain models for authentication and organization management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A registered user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An organization (may have a parent for hierarchy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Role within an organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrgRole {
    Owner,
    Admin,
    Member,
}

impl OrgRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrgRole::Owner => "owner",
            OrgRole::Admin => "admin",
            OrgRole::Member => "member",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(OrgRole::Owner),
            "admin" => Some(OrgRole::Admin),
            "member" => Some(OrgRole::Member),
            _ => None,
        }
    }

    /// Whether this role can manage members (add/remove/change roles)
    pub fn can_manage_members(&self) -> bool {
        matches!(self, OrgRole::Owner | OrgRole::Admin)
    }

    /// Whether this role can modify the organization itself
    pub fn can_modify_org(&self) -> bool {
        matches!(self, OrgRole::Owner | OrgRole::Admin)
    }

    /// Whether this role can delete the organization
    pub fn can_delete_org(&self) -> bool {
        matches!(self, OrgRole::Owner)
    }
}

/// A user's membership in an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMembership {
    pub user_id: String,
    pub org_id: String,
    pub role: OrgRole,
    pub created_at: DateTime<Utc>,
}

/// A system-to-organization association
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOrg {
    pub system_name: String,
    pub org_id: String,
    pub created_at: DateTime<Utc>,
}

/// Session ownership record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: String,
    pub system_name: String,
    pub created_at: DateTime<Utc>,
}

/// JWT claims for access tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthClaims {
    /// Subject (user ID)
    pub sub: String,
    /// User email
    pub email: String,
    /// Expiration time (unix timestamp)
    pub exp: usize,
    /// Issued at (unix timestamp)
    pub iat: usize,
}

/// User info returned to the frontend (no password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            created_at: user.created_at,
        }
    }
}

/// Organization with the user's role in it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgWithRole {
    #[serde(flatten)]
    pub org: Organization,
    pub role: OrgRole,
}

/// Organization member details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub role: OrgRole,
    pub joined_at: DateTime<Utc>,
}
