#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Admin,
    User,
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
        }
    }

    pub fn is_admin(self) -> bool {
        self == Self::Admin
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub access_token_hash: String,
    pub refresh_token_hash: String,
    pub access_expires_at: i64,
    pub refresh_expires_at: i64,
}
