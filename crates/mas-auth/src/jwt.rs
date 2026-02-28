//! JWT token creation and verification

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::models::AuthClaims;
use crate::AuthError;

/// Configuration for JWT token management
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    secret: String,
    /// Access token lifetime in seconds (default: 15 minutes)
    pub access_token_ttl_secs: u64,
    /// Refresh token lifetime in seconds (default: 7 days)
    pub refresh_token_ttl_secs: u64,
}

impl JwtConfig {
    /// Create a new JWT config with the given secret
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            access_token_ttl_secs: 15 * 60,         // 15 minutes
            refresh_token_ttl_secs: 7 * 24 * 60 * 60, // 7 days
        }
    }

    /// Create a config suitable for testing
    pub fn for_testing() -> Self {
        Self::new("test-secret-do-not-use-in-production".to_string())
    }

    /// Create an access token for the given user
    pub fn create_access_token(&self, user_id: &str, email: &str) -> Result<String, AuthError> {
        let now = Utc::now().timestamp() as usize;
        let claims = AuthClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: now + self.access_token_ttl_secs as usize,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AuthError::Internal(format!("Failed to create token: {}", e)))
    }

    /// Verify and decode an access token
    pub fn verify_access_token(&self, token: &str) -> Result<AuthClaims, AuthError> {
        decode::<AuthClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| AuthError::InvalidToken(format!("Token verification failed: {}", e)))
    }

    /// Create a refresh token (opaque random string)
    pub fn create_refresh_token(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// Hash a refresh token for storage (we never store the raw token)
pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Token pair returned after login/refresh
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let config = JwtConfig::for_testing();
        let token = config
            .create_access_token("user-123", "test@example.com")
            .unwrap();
        let claims = config.verify_access_token(&token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::for_testing();
        let result = config.verify_access_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_token_hashing() {
        let token = "some-refresh-token";
        let hash1 = hash_refresh_token(token);
        let hash2 = hash_refresh_token(token);
        assert_eq!(hash1, hash2); // Deterministic
        assert_ne!(hash1, hash_refresh_token("different-token"));
    }
}
