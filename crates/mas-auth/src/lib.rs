//! Authentication and organization management for the Multi-Agent System
//!
//! This crate provides:
//! - User registration and login (Argon2 password hashing + JWT tokens)
//! - Organization management with hierarchical structure
//! - Many-to-many user↔org membership with roles (owner/admin/member)
//! - System↔org associations for access control
//! - Session ownership tracking
//! - Axum middleware extractor for protected endpoints

pub mod db;
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod password;
pub mod repository;

use thiserror::Error;

/// Auth-related errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Email already taken: {0}")]
    EmailTaken(String),

    #[error("Organization slug already taken: {0}")]
    OrgSlugTaken(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// Re-export commonly used types
pub use db::{create_pool, run_migrations};
pub use jwt::{JwtConfig, TokenPair};
pub use middleware::{AuthState, AuthenticatedUser, FromRef, OptionalUser};
pub use models::{
    AuthClaims, MemberInfo, OrgMembership, OrgRole, OrgWithRole, Organization, SessionRecord,
    SystemOrg, User, UserInfo,
};
