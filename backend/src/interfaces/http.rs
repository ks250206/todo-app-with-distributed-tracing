use axum::{
    Json, Router,
    extract::{Extension, MatchedPath, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use opentelemetry::{
    global,
    trace::{Status, TraceContextExt},
};
use opentelemetry_http::HeaderExtractor;
use serde::{Deserialize, Serialize};
use tracing::{Instrument, field};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{
    application::{AuthService, AuthTokens, ServiceError, TodoService},
    domain::{Todo, User, UserRole},
};

use super::rate_limit::AuthRateLimiter;

const ACCESS_COOKIE: &str = "access_token";
const REFRESH_COOKIE: &str = "refresh_token";
const ACCESS_MAX_AGE: i64 = 15 * 60;
const REFRESH_MAX_AGE: i64 = 30 * 24 * 60 * 60;

#[derive(Clone)]
pub struct AppState {
    auth: AuthService,
    todos: TodoService,
    auth_rate_limiter: AuthRateLimiter,
}
#[derive(Clone)]
struct AuthenticatedUser {
    id: i64,
    email: String,
    role: UserRole,
}
#[derive(Deserialize)]
struct CredentialsRequest {
    email: String,
    password: String,
}
#[derive(Deserialize)]
struct CreateTodoRequest {
    title: String,
}
#[derive(Deserialize)]
struct UpdateTodoRequest {
    title: Option<String>,
    completed: Option<bool>,
}
#[derive(Serialize)]
struct UserResponse {
    id: i64,
    email: String,
    role: String,
}
#[derive(Serialize)]
struct TodoResponse {
    id: i64,
    title: String,
    completed: bool,
    created_at: i64,
    updated_at: i64,
}
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}
struct HttpError(ServiceError);

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            role: user.role.as_str().to_owned(),
        }
    }
}
impl From<Todo> for TodoResponse {
    fn from(todo: Todo) -> Self {
        Self {
            id: todo.id,
            title: todo.title,
            completed: todo.completed,
            created_at: todo.created_at,
            updated_at: todo.updated_at,
        }
    }
}
impl From<ServiceError> for HttpError {
    fn from(error: ServiceError) -> Self {
        Self(error)
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            ServiceError::Validation(message) => (StatusCode::BAD_REQUEST, message),
            ServiceError::InvalidCredentials | ServiceError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "unauthorized".to_owned())
            }
            ServiceError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_owned()),
            ServiceError::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
            ServiceError::Conflict => (StatusCode::CONFLICT, "already exists".to_owned()),
            ServiceError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "too many requests".to_owned(),
            ),
            ServiceError::Internal(error) => {
                tracing::error!(error = %error, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };
        let mut response = (status, Json(ErrorBody { error: message })).into_response();
        if status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        }
        response
    }
}

pub fn router(auth: AuthService, todos: TodoService) -> Router {
    let state = AppState {
        auth,
        todos,
        auth_rate_limiter: AuthRateLimiter::default(),
    };
    let protected = Router::new()
        .route("/api/me", get(me))
        .route("/api/internal/admin-auth", get(admin_auth))
        .route("/api/internal/session-auth", get(session_auth))
        .route("/api/todos", get(list_todos).post(create_todo))
        .route(
            "/api/todos/{id}",
            get(get_todo).patch(update_todo).delete(delete_todo),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .merge(protected)
        .with_state(state)
        .layer(middleware::from_fn(http_trace))
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CredentialsRequest>,
) -> Result<impl IntoResponse, HttpError> {
    if !state
        .auth_rate_limiter
        .check_register(&client_source(&headers), &account_key(&input.email))
    {
        return Err(ServiceError::RateLimited.into());
    }
    let (user, tokens) = state.auth.register(input.email, input.password).await?;
    Ok((
        StatusCode::CREATED,
        session_headers(&tokens),
        Json(UserResponse::from(user)),
    ))
}
async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CredentialsRequest>,
) -> Result<impl IntoResponse, HttpError> {
    if !state
        .auth_rate_limiter
        .check_login(&client_source(&headers), &account_key(&input.email))
    {
        return Err(ServiceError::RateLimited.into());
    }
    let (user, tokens) = state.auth.login(input.email, input.password).await?;
    Ok((session_headers(&tokens), Json(UserResponse::from(user))))
}
async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, HttpError> {
    let refresh = cookie(&headers, REFRESH_COOKIE).ok_or(ServiceError::Unauthorized)?;
    let tokens = state.auth.refresh(refresh).await?;
    Ok((session_headers(&tokens), StatusCode::NO_CONTENT))
}
async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, HttpError> {
    state
        .auth
        .logout(
            cookie(&headers, ACCESS_COOKIE),
            cookie(&headers, REFRESH_COOKIE),
        )
        .await?;
    Ok((clear_session_headers(), StatusCode::NO_CONTENT))
}
async fn me(
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<UserResponse>, HttpError> {
    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        role: user.role.as_str().to_owned(),
    }))
}
async fn admin_auth(
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<StatusCode, HttpError> {
    if user.role.is_admin() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ServiceError::Forbidden.into())
    }
}
async fn session_auth() -> StatusCode {
    StatusCode::NO_CONTENT
}
async fn create_todo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(input): Json<CreateTodoRequest>,
) -> Result<impl IntoResponse, HttpError> {
    let todo = state.todos.create(user.id, input.title).await?;
    Ok((StatusCode::CREATED, Json(TodoResponse::from(todo))))
}
async fn list_todos(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<TodoResponse>>, HttpError> {
    Ok(Json(
        state
            .todos
            .list(user.id)
            .await?
            .into_iter()
            .map(TodoResponse::from)
            .collect(),
    ))
}
async fn get_todo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i64>,
) -> Result<Json<TodoResponse>, HttpError> {
    Ok(Json(TodoResponse::from(
        state.todos.get(user.id, id).await?,
    )))
}
async fn update_todo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateTodoRequest>,
) -> Result<Json<TodoResponse>, HttpError> {
    Ok(Json(TodoResponse::from(
        state
            .todos
            .update(user.id, id, input.title, input.completed)
            .await?,
    )))
}
async fn delete_todo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i64>,
) -> Result<StatusCode, HttpError> {
    state.todos.delete(user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, HttpError> {
    let access_token =
        cookie(request.headers(), ACCESS_COOKIE).ok_or(ServiceError::Unauthorized)?;
    let user = state.auth.authenticate(access_token).await?;
    request.extensions_mut().insert(AuthenticatedUser {
        id: user.id,
        email: user.email,
        role: user.role,
    });
    Ok(next.run(request).await)
}
fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&format!("{name}=")))
}
fn client_source(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .and_then(|value| value.parse::<std::net::IpAddr>().ok())
        .map(|address| address.to_string())
        .unwrap_or_else(|| "loopback".to_owned())
}
fn account_key(email: &str) -> String {
    let mut account = email.trim().to_ascii_lowercase();
    account.truncate(320);
    account
}
fn session_headers(tokens: &AuthTokens) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        cookie_header(ACCESS_COOKIE, "", 0, "/api"),
    );
    headers.append(
        header::SET_COOKIE,
        cookie_header(REFRESH_COOKIE, "", 0, "/api"),
    );
    headers.append(
        header::SET_COOKIE,
        cookie_header(ACCESS_COOKIE, &tokens.access_token, ACCESS_MAX_AGE, "/"),
    );
    headers.append(
        header::SET_COOKIE,
        cookie_header(REFRESH_COOKIE, &tokens.refresh_token, REFRESH_MAX_AGE, "/"),
    );
    headers
}
fn clear_session_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    for path in ["/", "/api"] {
        headers.append(
            header::SET_COOKIE,
            cookie_header(ACCESS_COOKIE, "", 0, path),
        );
        headers.append(
            header::SET_COOKIE,
            cookie_header(REFRESH_COOKIE, "", 0, path),
        );
    }
    headers
}
fn cookie_header(name: &str, value: &str, max_age: i64, path: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{name}={value}; Path={path}; Max-Age={max_age}; HttpOnly; Secure; SameSite=Lax"
    ))
    .expect("token is safe for cookie header")
}

async fn http_trace(req: Request, next: Next) -> Response {
    let started_at = std::time::Instant::now();
    let method = req.method().clone();
    let url_path = req.uri().path().to_owned();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or("UNMATCHED")
        .to_owned();
    let span_name = if route == "UNMATCHED" {
        method.to_string()
    } else {
        format!("{method} {route}")
    };
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });
    let span = tracing::info_span!("http.server", otel.name = %span_name, otel.kind = "server", trace_id = field::Empty, span_id = field::Empty, "http.request.method" = %method, "http.route" = %route, "url.path" = %url_path, "url.scheme" = "https", "http.response.status_code" = field::Empty, "error.type" = field::Empty);
    if let Err(error) = span.set_parent(parent_context) {
        tracing::warn!(%error, "failed to set remote trace parent");
    }
    let otel_context = span.context();
    let otel_span = otel_context.span();
    let span_context = otel_span.span_context();
    span.record("trace_id", span_context.trace_id().to_string());
    span.record("span_id", span_context.span_id().to_string());
    let response = next.run(req).instrument(span.clone()).await;
    let status = response.status();
    span.record("http.response.status_code", status.as_u16() as u64);
    if status.is_server_error() {
        let error_type = status.as_u16().to_string();
        span.set_attribute("error.type", error_type.clone());
        span.set_status(Status::error(error_type));
    }
    span.in_scope(|| {
        tracing::info!(
            "http.response.status_code" = status.as_u16(),
            duration_ms = started_at.elapsed().as_secs_f64() * 1_000.0,
            "request completed"
        );
    });
    response
}
