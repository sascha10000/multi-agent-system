//! Route handlers for the REST API

pub mod auth;
pub mod orgs;
pub mod sessions;
pub mod systems;

// Re-export all handlers for backward compatibility
pub use auth::*;
pub use orgs::*;
pub use sessions::*;
pub use systems::*;
