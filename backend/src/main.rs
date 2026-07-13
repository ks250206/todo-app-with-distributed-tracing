mod application;
mod domain;
mod infrastructure;
mod interfaces;
mod observability;

use anyhow::Context as _;
use application::{AuthService, TodoService};
use infrastructure::SqliteRepository;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let tracer_provider = observability::init_tracing()?;
    let pepper = std::env::var("PASSWORD_PEPPER").context(
        "PASSWORD_PEPPER must be set; copy .env.example to .env and set a long random secret",
    )?;
    if pepper.len() < 32 {
        anyhow::bail!("PASSWORD_PEPPER must be at least 32 characters");
    }
    let repository = SqliteRepository::connect("sqlite://app.db?mode=rwc")
        .await
        .context("failed to connect to SQLite")?;
    repository
        .migrate()
        .await
        .context("failed to migrate database")?;

    let auth = AuthService::new(repository.clone(), pepper)
        .context("failed to initialize authentication service")?;
    let app = interfaces::router(auth, TodoService::new(repository));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .context("failed to bind server")?;

    let address = listener.local_addr()?;
    tracing::info!(%address, "server started");

    let server_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    let shutdown_result = tracer_provider.shutdown();

    server_result.context("Axum server failed")?;
    shutdown_result.context("failed to shut down tracer provider")?;

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::error!(%error, "failed to install SIGTERM handler");
                wait_for_ctrl_c().await;
                return;
            }
        };

        tokio::select! {
            _ = wait_for_ctrl_c() => {}
            _ = terminate.recv() => tracing::info!("received SIGTERM"),
        }
    }

    #[cfg(not(unix))]
    wait_for_ctrl_c().await;
}

async fn wait_for_ctrl_c() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to install Ctrl+C handler");
    }
}
