use std::str::FromStr;

use async_trait::async_trait;
use sqlx::{
    FromRow, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use crate::domain::{
    NewTodo, RepositoryError, Session, SessionRepository, Todo, TodoPatch, TodoRepository, User,
    UserRepository, UserRole,
};

#[derive(Clone)]
pub struct SqliteRepository {
    pool: SqlitePool,
}

#[derive(FromRow)]
struct UserRow {
    id: i64,
    email: String,
    password_hash: Option<String>,
    role: String,
}
#[derive(FromRow)]
struct SessionRow {
    id: String,
    user_id: i64,
    access_token_hash: String,
    refresh_token_hash: String,
    access_expires_at: i64,
    refresh_expires_at: i64,
}
#[derive(FromRow)]
struct TodoRow {
    id: i64,
    title: String,
    completed: i64,
    created_at: i64,
    updated_at: i64,
}
impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            password_hash: row.password_hash.unwrap_or_default(),
            role: match row.role.as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::User,
            },
        }
    }
}
impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            access_token_hash: row.access_token_hash,
            refresh_token_hash: row.refresh_token_hash,
            access_expires_at: row.access_expires_at,
            refresh_expires_at: row.refresh_expires_at,
        }
    }
}
impl From<TodoRow> for Todo {
    fn from(row: TodoRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            completed: row.completed != 0,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl SqliteRepository {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(database_url)?.foreign_keys(true);
        Ok(Self {
            pool: SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(options)
                .await?,
        })
    }
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE, password_hash TEXT, role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('admin', 'user')))").execute(&self.pool).await?;
        let has_password_hash = sqlx::query_scalar::<_, String>(
            "SELECT name FROM pragma_table_info('users') WHERE name = 'password_hash'",
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some();
        if !has_password_hash {
            sqlx::query("ALTER TABLE users ADD COLUMN password_hash TEXT")
                .execute(&self.pool)
                .await?;
        }
        let has_role = sqlx::query_scalar::<_, String>(
            "SELECT name FROM pragma_table_info('users') WHERE name = 'role'",
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some();
        if !has_role {
            sqlx::query("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'")
                .execute(&self.pool)
                .await?;
            sqlx::query("UPDATE users SET role = 'admin' WHERE id = (SELECT MIN(id) FROM users)")
                .execute(&self.pool)
                .await?;
        }
        sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS users_single_admin ON users(role) WHERE role = 'admin'")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, access_token_hash TEXT NOT NULL UNIQUE, refresh_token_hash TEXT NOT NULL UNIQUE, access_expires_at INTEGER NOT NULL, refresh_expires_at INTEGER NOT NULL)").execute(&self.pool).await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS todos (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE, title TEXT NOT NULL, completed INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS todos_user_id_id ON todos(user_id, id)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl UserRepository for SqliteRepository {
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<User, RepositoryError> {
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(map_error)?;
        let role = sqlx::query_scalar::<_, String>(
            "SELECT CASE WHEN EXISTS (SELECT 1 FROM users) THEN 'user' ELSE 'admin' END",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_error)?;
        let user = sqlx::query_as::<_, UserRow>("INSERT INTO users (name, email, password_hash, role) VALUES (?, ?, ?, ?) RETURNING id, email, password_hash, role")
            .bind(email).bind(email).bind(password_hash).bind(role).fetch_one(&mut *transaction).await.map(User::from).map_err(map_error)?;
        transaction.commit().await.map_err(map_error)?;
        Ok(user)
    }
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError> {
        sqlx::query_as::<_, UserRow>(
            "SELECT id, email, password_hash, role FROM users WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(User::from))
        .map_err(map_error)
    }
    async fn find_user_by_id(&self, id: i64) -> Result<Option<User>, RepositoryError> {
        sqlx::query_as::<_, UserRow>(
            "SELECT id, email, password_hash, role FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(User::from))
        .map_err(map_error)
    }
}

#[async_trait]
impl SessionRepository for SqliteRepository {
    async fn create_session(&self, session: &Session) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO sessions (id, user_id, access_token_hash, refresh_token_hash, access_expires_at, refresh_expires_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&session.id).bind(session.user_id).bind(&session.access_token_hash).bind(&session.refresh_token_hash).bind(session.access_expires_at).bind(session.refresh_expires_at).execute(&self.pool).await.map(|_| ()).map_err(map_error)
    }
    async fn find_session_by_access_hash(
        &self,
        hash: &str,
    ) -> Result<Option<Session>, RepositoryError> {
        find_session(&self.pool, "access_token_hash", hash).await
    }
    async fn find_session_by_refresh_hash(
        &self,
        hash: &str,
    ) -> Result<Option<Session>, RepositoryError> {
        find_session(&self.pool, "refresh_token_hash", hash).await
    }
    async fn rotate_session(
        &self,
        session: &Session,
        previous_refresh_token_hash: &str,
    ) -> Result<bool, RepositoryError> {
        sqlx::query("UPDATE sessions SET access_token_hash = ?, refresh_token_hash = ?, access_expires_at = ?, refresh_expires_at = ? WHERE id = ? AND refresh_token_hash = ?")
            .bind(&session.access_token_hash).bind(&session.refresh_token_hash).bind(session.access_expires_at).bind(session.refresh_expires_at).bind(&session.id).bind(previous_refresh_token_hash).execute(&self.pool).await.map(|result| result.rows_affected() == 1).map_err(map_error)
    }
    async fn delete_session(&self, id: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(map_error)
    }
}

#[async_trait]
impl TodoRepository for SqliteRepository {
    async fn create_todo(
        &self,
        user_id: i64,
        todo: NewTodo,
        now: i64,
    ) -> Result<Todo, RepositoryError> {
        sqlx::query_as::<_, TodoRow>("INSERT INTO todos (user_id, title, completed, created_at, updated_at) VALUES (?, ?, 0, ?, ?) RETURNING id, title, completed, created_at, updated_at")
            .bind(user_id).bind(todo.title()).bind(now).bind(now).fetch_one(&self.pool).await.map(Todo::from).map_err(map_error)
    }
    async fn list_todos(&self, user_id: i64) -> Result<Vec<Todo>, RepositoryError> {
        sqlx::query_as::<_, TodoRow>("SELECT id, title, completed, created_at, updated_at FROM todos WHERE user_id = ? ORDER BY id DESC").bind(user_id).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(Todo::from).collect()).map_err(map_error)
    }
    async fn find_todo(&self, user_id: i64, todo_id: i64) -> Result<Option<Todo>, RepositoryError> {
        sqlx::query_as::<_, TodoRow>("SELECT id, title, completed, created_at, updated_at FROM todos WHERE id = ? AND user_id = ?").bind(todo_id).bind(user_id).fetch_optional(&self.pool).await.map(|row| row.map(Todo::from)).map_err(map_error)
    }
    async fn update_todo(
        &self,
        user_id: i64,
        todo_id: i64,
        patch: TodoPatch,
        now: i64,
    ) -> Result<Option<Todo>, RepositoryError> {
        sqlx::query_as::<_, TodoRow>("UPDATE todos SET title = COALESCE(?, title), completed = COALESCE(?, completed), updated_at = ? WHERE id = ? AND user_id = ? RETURNING id, title, completed, created_at, updated_at")
            .bind(patch.title).bind(patch.completed).bind(now).bind(todo_id).bind(user_id).fetch_optional(&self.pool).await.map(|row| row.map(Todo::from)).map_err(map_error)
    }
    async fn delete_todo(&self, user_id: i64, todo_id: i64) -> Result<bool, RepositoryError> {
        sqlx::query("DELETE FROM todos WHERE id = ? AND user_id = ?")
            .bind(todo_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() == 1)
            .map_err(map_error)
    }
}

async fn find_session(
    pool: &SqlitePool,
    column: &str,
    hash: &str,
) -> Result<Option<Session>, RepositoryError> {
    let query = match column {
        "access_token_hash" => {
            "SELECT id, user_id, access_token_hash, refresh_token_hash, access_expires_at, refresh_expires_at FROM sessions WHERE access_token_hash = ?"
        }
        "refresh_token_hash" => {
            "SELECT id, user_id, access_token_hash, refresh_token_hash, access_expires_at, refresh_expires_at FROM sessions WHERE refresh_token_hash = ?"
        }
        _ => unreachable!("fixed session token column"),
    };
    sqlx::query_as::<_, SessionRow>(query)
        .bind(hash)
        .fetch_optional(pool)
        .await
        .map(|row| row.map(Session::from))
        .map_err(map_error)
}
fn map_error(error: sqlx::Error) -> RepositoryError {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        RepositoryError::Conflict
    } else {
        RepositoryError::Unexpected(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AuthService, ServiceError};

    #[tokio::test]
    async fn expired_access_token_does_not_invalidate_refresh_token() {
        let repository = SqliteRepository::connect("sqlite::memory:")
            .await
            .expect("connect to SQLite");
        repository.migrate().await.expect("migrate database");
        let auth = AuthService::new(repository.clone(), "p".repeat(32)).unwrap();
        let (_, tokens) = auth
            .register(
                "owner@example.test".to_owned(),
                "long-enough-password".to_owned(),
            )
            .await
            .unwrap();
        sqlx::query("UPDATE sessions SET access_expires_at = 0")
            .execute(&repository.pool)
            .await
            .unwrap();

        assert!(matches!(
            auth.authenticate(&tokens.access_token).await,
            Err(ServiceError::Unauthorized)
        ));
        assert!(auth.refresh(&tokens.refresh_token).await.is_ok());
    }

    #[tokio::test]
    async fn only_the_first_user_is_admin() {
        let repository = SqliteRepository::connect("sqlite::memory:")
            .await
            .expect("connect to SQLite");
        repository.migrate().await.expect("migrate database");

        let first = repository
            .create_user("first@example.test", "hash")
            .await
            .expect("create first user");
        let second = repository
            .create_user("second@example.test", "hash")
            .await
            .expect("create second user");

        assert_eq!(first.role, UserRole::Admin);
        assert_eq!(second.role, UserRole::User);
    }

    #[tokio::test]
    async fn todo_queries_are_scoped_to_the_owner() {
        let repository = SqliteRepository::connect("sqlite::memory:")
            .await
            .expect("connect to SQLite");
        repository.migrate().await.expect("migrate database");
        let owner = repository
            .create_user("owner@example.test", "hash")
            .await
            .expect("create owner");
        let other = repository
            .create_user("other@example.test", "hash")
            .await
            .expect("create other user");
        let todo = repository
            .create_todo(owner.id, NewTodo::new("private".to_owned()).unwrap(), 1)
            .await
            .expect("create todo");

        assert!(repository.list_todos(other.id).await.unwrap().is_empty());
        assert!(
            repository
                .find_todo(other.id, todo.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            repository
                .update_todo(
                    other.id,
                    todo.id,
                    TodoPatch::new(Some("changed".to_owned()), None).unwrap(),
                    2,
                )
                .await
                .unwrap()
                .is_none()
        );
        assert!(!repository.delete_todo(other.id, todo.id).await.unwrap());
        assert_eq!(
            repository
                .find_todo(owner.id, todo.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "private"
        );
    }

    #[tokio::test]
    async fn partial_todo_updates_preserve_unspecified_fields() {
        let repository = SqliteRepository::connect("sqlite::memory:")
            .await
            .expect("connect to SQLite");
        repository.migrate().await.expect("migrate database");
        let owner = repository
            .create_user("owner@example.test", "hash")
            .await
            .expect("create owner");
        let todo = repository
            .create_todo(owner.id, NewTodo::new("original".to_owned()).unwrap(), 1)
            .await
            .expect("create todo");

        repository
            .update_todo(
                owner.id,
                todo.id,
                TodoPatch::new(Some("renamed".to_owned()), None).unwrap(),
                2,
            )
            .await
            .unwrap();
        let updated = repository
            .update_todo(
                owner.id,
                todo.id,
                TodoPatch::new(None, Some(true)).unwrap(),
                3,
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.title, "renamed");
        assert!(updated.completed);
    }

    #[tokio::test]
    async fn foreign_keys_cascade_user_deletions() {
        let repository = SqliteRepository::connect("sqlite::memory:")
            .await
            .expect("connect to SQLite");
        repository.migrate().await.expect("migrate database");
        let user = repository
            .create_user("owner@example.test", "hash")
            .await
            .expect("create owner");
        repository
            .create_todo(user.id, NewTodo::new("private".to_owned()).unwrap(), 1)
            .await
            .expect("create todo");
        repository
            .create_session(&Session {
                id: "session".to_owned(),
                user_id: user.id,
                access_token_hash: "access".to_owned(),
                refresh_token_hash: "refresh".to_owned(),
                access_expires_at: 10,
                refresh_expires_at: 20,
            })
            .await
            .expect("create session");

        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user.id)
            .execute(&repository.pool)
            .await
            .unwrap();

        let todo_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM todos")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
            .fetch_one(&repository.pool)
            .await
            .unwrap();
        assert_eq!(todo_count, 0);
        assert_eq!(session_count, 0);
    }
}
