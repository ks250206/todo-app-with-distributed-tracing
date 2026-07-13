use std::error::Error;

use async_trait::async_trait;
use thiserror::Error;

use super::{NewTodo, Session, Todo, TodoPatch, User};

pub type BoxError = Box<dyn Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("already exists")]
    Conflict,
    #[error("repository operation failed")]
    Unexpected(#[source] BoxError),
}

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<User, RepositoryError>;
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError>;
    async fn find_user_by_id(&self, id: i64) -> Result<Option<User>, RepositoryError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync + 'static {
    async fn create_session(&self, session: &Session) -> Result<(), RepositoryError>;
    async fn find_session_by_access_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Session>, RepositoryError>;
    async fn find_session_by_refresh_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Session>, RepositoryError>;
    async fn rotate_session(
        &self,
        session: &Session,
        previous_refresh_token_hash: &str,
    ) -> Result<bool, RepositoryError>;
    async fn delete_session(&self, id: &str) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait TodoRepository: Send + Sync + 'static {
    async fn create_todo(
        &self,
        user_id: i64,
        todo: NewTodo,
        now: i64,
    ) -> Result<Todo, RepositoryError>;
    async fn list_todos(&self, user_id: i64) -> Result<Vec<Todo>, RepositoryError>;
    async fn find_todo(&self, user_id: i64, todo_id: i64) -> Result<Option<Todo>, RepositoryError>;
    async fn update_todo(
        &self,
        user_id: i64,
        todo_id: i64,
        patch: TodoPatch,
        now: i64,
    ) -> Result<Option<Todo>, RepositoryError>;
    async fn delete_todo(&self, user_id: i64, todo_id: i64) -> Result<bool, RepositoryError>;
}
