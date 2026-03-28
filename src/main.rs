//! Bastion - macOS AI Agent Sandbox
//!
//! A secure execution environment for AI agents on macOS.

use std::path::PathBuf;

use bastion::{create_app, create_state, BastionMcpServer, Config};
use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn init_logging(log_dir: &PathBuf) {
    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "bastion.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::new("info"))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();

    std::mem::forget(_guard);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::default();

    // Ensure directories exist
    std::fs::create_dir_all(&config.log_dir).ok();
    std::fs::create_dir_all(&config.data_dir).ok();
    std::fs::create_dir_all(&config.profiles_dir).ok();

    // Initialize logging
    init_logging(&config.log_dir);

    let state = create_state(config.clone());

    if std::env::args().any(|a| a == "--mcp") {
        // MCP mode: JSON-RPC 2.0 over STDIO
        info!("Starting Bastion MCP server v{}", "0.1.0");
        let server = BastionMcpServer::new(state);
        server.run().await
    } else {
        // HTTP mode
        info!("Starting Bastion v{}", "0.1.0");
        info!("Listening on {}:{}", config.host, config.port);

        let addr: std::net::SocketAddr = format!("{}:{}", config.host, config.port)
            .parse()
            .expect("Invalid address");

        let app = create_app(state);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}
