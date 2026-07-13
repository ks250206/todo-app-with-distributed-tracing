use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewTodo {
    title: String,
}

#[derive(Debug, Clone)]
pub struct TodoPatch {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Debug, Error)]
pub enum TodoValidationError {
    #[error("title is required")]
    EmptyTitle,
    #[error("title is too long")]
    TitleTooLong,
    #[error("at least one field is required")]
    EmptyPatch,
}

impl NewTodo {
    pub fn new(title: String) -> Result<Self, TodoValidationError> {
        let title = title.trim().to_owned();
        if title.is_empty() {
            return Err(TodoValidationError::EmptyTitle);
        }
        if title.chars().count() > 500 {
            return Err(TodoValidationError::TitleTooLong);
        }
        Ok(Self { title })
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

impl TodoPatch {
    pub fn new(
        title: Option<String>,
        completed: Option<bool>,
    ) -> Result<Self, TodoValidationError> {
        let title = title.map(|title| title.trim().to_owned());
        if title.as_deref().is_some_and(str::is_empty) {
            return Err(TodoValidationError::EmptyTitle);
        }
        if title
            .as_ref()
            .is_some_and(|title| title.chars().count() > 500)
        {
            return Err(TodoValidationError::TitleTooLong);
        }
        if title.is_none() && completed.is_none() {
            return Err(TodoValidationError::EmptyPatch);
        }
        Ok(Self { title, completed })
    }
}
