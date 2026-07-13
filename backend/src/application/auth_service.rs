use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

use crate::domain::{RepositoryError, Session, SessionRepository, User, UserRepository};

const ACCESS_TOKEN_TTL_SECONDS: i64 = 15 * 60;
const REFRESH_TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("too many requests")]
    RateLimited,
    #[error("internal service error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Clone)]
pub struct AuthService {
    repository: Arc<dyn UserRepository + Send + Sync>,
    sessions: Arc<dyn SessionRepository + Send + Sync>,
    pepper: Arc<str>,
    dummy_password_hash: Arc<str>,
}

impl AuthService {
    pub fn new<R>(repository: R, pepper: String) -> Result<Self, ServiceError>
    where
        R: UserRepository + SessionRepository,
    {
        let dummy_password_hash = hash_password("dummy-password-for-timing", &pepper)?;
        let repository = Arc::new(repository);
        Ok(Self {
            repository: repository.clone(),
            sessions: repository,
            pepper: Arc::from(pepper),
            dummy_password_hash: Arc::from(dummy_password_hash),
        })
    }

    #[tracing::instrument(name = "auth.register", skip_all)]
    pub async fn register(
        &self,
        email: String,
        password: String,
    ) -> Result<(User, AuthTokens), ServiceError> {
        let email = normalize_email(email)?;
        validate_password(&password)?;
        let password_hash = hash_password(&password, &self.pepper)?;
        let user = self
            .repository
            .create_user(&email, &password_hash)
            .await
            .map_err(map_repository_error)?;
        let tokens = self.create_session(user.id).await?;
        Ok((user, tokens))
    }

    #[tracing::instrument(name = "auth.login", skip_all)]
    pub async fn login(
        &self,
        email: String,
        password: String,
    ) -> Result<(User, AuthTokens), ServiceError> {
        let email = normalize_email(email)?;
        let user = self
            .repository
            .find_user_by_email(&email)
            .await
            .map_err(map_repository_error)?;
        let Some(user) = user else {
            let _ = verify_password(&password, &self.pepper, &self.dummy_password_hash);
            return Err(ServiceError::InvalidCredentials);
        };
        verify_password(&password, &self.pepper, &user.password_hash)?;
        let tokens = self.create_session(user.id).await?;
        Ok((user, tokens))
    }

    #[tracing::instrument(name = "auth.authenticate", skip_all)]
    pub async fn authenticate(&self, access_token: &str) -> Result<User, ServiceError> {
        let session = self
            .sessions
            .find_session_by_access_hash(&token_hash(access_token))
            .await
            .map_err(map_repository_error)?
            .ok_or(ServiceError::Unauthorized)?;
        if session.access_expires_at <= now_epoch_seconds() {
            return Err(ServiceError::Unauthorized);
        }
        self.repository
            .find_user_by_id(session.user_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(ServiceError::Unauthorized)
    }

    #[tracing::instrument(name = "auth.refresh", skip_all)]
    pub async fn refresh(&self, refresh_token: &str) -> Result<AuthTokens, ServiceError> {
        let previous_refresh_token_hash = token_hash(refresh_token);
        let previous = self
            .sessions
            .find_session_by_refresh_hash(&previous_refresh_token_hash)
            .await
            .map_err(map_repository_error)?
            .ok_or(ServiceError::Unauthorized)?;
        if previous.refresh_expires_at <= now_epoch_seconds() {
            self.sessions
                .delete_session(&previous.id)
                .await
                .map_err(map_repository_error)?;
            return Err(ServiceError::Unauthorized);
        }
        let tokens = new_tokens();
        let now = now_epoch_seconds();
        let session = Session {
            id: previous.id,
            user_id: previous.user_id,
            access_token_hash: token_hash(&tokens.access_token),
            refresh_token_hash: token_hash(&tokens.refresh_token),
            access_expires_at: now + ACCESS_TOKEN_TTL_SECONDS,
            refresh_expires_at: now + REFRESH_TOKEN_TTL_SECONDS,
        };
        if !self
            .sessions
            .rotate_session(&session, &previous_refresh_token_hash)
            .await
            .map_err(map_repository_error)?
        {
            return Err(ServiceError::Unauthorized);
        }
        Ok(tokens)
    }

    pub async fn logout(
        &self,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
    ) -> Result<(), ServiceError> {
        let session = if let Some(token) = refresh_token {
            self.sessions
                .find_session_by_refresh_hash(&token_hash(token))
                .await
        } else if let Some(token) = access_token {
            self.sessions
                .find_session_by_access_hash(&token_hash(token))
                .await
        } else {
            return Ok(());
        }
        .map_err(map_repository_error)?;
        if let Some(session) = session {
            self.sessions
                .delete_session(&session.id)
                .await
                .map_err(map_repository_error)?;
        }
        Ok(())
    }

    async fn create_session(&self, user_id: i64) -> Result<AuthTokens, ServiceError> {
        let tokens = new_tokens();
        let now = now_epoch_seconds();
        self.sessions
            .create_session(&Session {
                id: random_token(24),
                user_id,
                access_token_hash: token_hash(&tokens.access_token),
                refresh_token_hash: token_hash(&tokens.refresh_token),
                access_expires_at: now + ACCESS_TOKEN_TTL_SECONDS,
                refresh_expires_at: now + REFRESH_TOKEN_TTL_SECONDS,
            })
            .await
            .map_err(map_repository_error)?;
        Ok(tokens)
    }
}

fn normalize_email(email: String) -> Result<String, ServiceError> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') || email.len() > 320 {
        return Err(ServiceError::Validation("email is invalid".to_owned()));
    }
    Ok(email)
}
fn validate_password(password: &str) -> Result<(), ServiceError> {
    if password.chars().count() < 12 {
        return Err(ServiceError::Validation(
            "password must be at least 12 characters".to_owned(),
        ));
    }
    Ok(())
}
fn pepper_password(password: &str, pepper: &str) -> Result<Vec<u8>, ServiceError> {
    let mut mac = HmacSha256::new_from_slice(pepper.as_bytes()).map_err(|error| {
        ServiceError::Internal(Box::new(std::io::Error::other(error.to_string())))
    })?;
    mac.update(password.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}
fn hash_password(password: &str, pepper: &str) -> Result<String, ServiceError> {
    let peppered = pepper_password(password, pepper)?;
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());
    argon2
        .hash_password(&peppered, &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| ServiceError::Internal(Box::new(std::io::Error::other(error.to_string()))))
}
fn verify_password(password: &str, pepper: &str, hash: &str) -> Result<(), ServiceError> {
    let peppered = pepper_password(password, pepper)?;
    let hash = PasswordHash::new(hash).map_err(|_| ServiceError::InvalidCredentials)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
        .verify_password(&peppered, &hash)
        .map_err(|_| ServiceError::InvalidCredentials)
}
fn new_tokens() -> AuthTokens {
    AuthTokens {
        access_token: random_token(32),
        refresh_token: random_token(48),
    }
}
fn random_token(bytes: usize) -> String {
    let mut value = vec![0; bytes];
    OsRng.fill_bytes(&mut value);
    URL_SAFE_NO_PAD.encode(value)
}
fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs() as i64
}
fn map_repository_error(error: RepositoryError) -> ServiceError {
    match error {
        RepositoryError::Conflict => ServiceError::Conflict,
        RepositoryError::Unexpected(error) => ServiceError::Internal(error),
    }
}
