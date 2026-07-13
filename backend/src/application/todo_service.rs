use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    application::ServiceError,
    domain::{NewTodo, Todo, TodoPatch, TodoRepository},
};

#[derive(Clone)]
pub struct TodoService {
    repository: Arc<dyn TodoRepository>,
}

impl TodoService {
    pub fn new<R: TodoRepository>(repository: R) -> Self {
        Self {
            repository: Arc::new(repository),
        }
    }
    pub async fn create(&self, user_id: i64, title: String) -> Result<Todo, ServiceError> {
        let todo = NewTodo::new(title).map_err(|e| ServiceError::Validation(e.to_string()))?;
        self.repository
            .create_todo(user_id, todo, now())
            .await
            .map_err(map_error)
    }
    pub async fn list(&self, user_id: i64) -> Result<Vec<Todo>, ServiceError> {
        self.repository.list_todos(user_id).await.map_err(map_error)
    }
    pub async fn get(&self, user_id: i64, todo_id: i64) -> Result<Todo, ServiceError> {
        self.repository
            .find_todo(user_id, todo_id)
            .await
            .map_err(map_error)?
            .ok_or(ServiceError::NotFound)
    }
    pub async fn update(
        &self,
        user_id: i64,
        todo_id: i64,
        title: Option<String>,
        completed: Option<bool>,
    ) -> Result<Todo, ServiceError> {
        let patch = TodoPatch::new(title, completed)
            .map_err(|e| ServiceError::Validation(e.to_string()))?;
        self.repository
            .update_todo(user_id, todo_id, patch, now())
            .await
            .map_err(map_error)?
            .ok_or(ServiceError::NotFound)
    }
    pub async fn delete(&self, user_id: i64, todo_id: i64) -> Result<(), ServiceError> {
        self.repository
            .delete_todo(user_id, todo_id)
            .await
            .map_err(map_error)?
            .then_some(())
            .ok_or(ServiceError::NotFound)
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs() as i64
}
fn map_error(error: crate::domain::RepositoryError) -> ServiceError {
    match error {
        crate::domain::RepositoryError::Conflict => ServiceError::Conflict,
        crate::domain::RepositoryError::Unexpected(error) => ServiceError::Internal(error),
    }
}
