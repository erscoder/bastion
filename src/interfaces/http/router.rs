use axum::{routing::{delete, get, post}, Router};
use tower_http::cors::{Any, CorsLayer};
use super::state::AppState;
use super::auth::auth_middleware;
use super::handlers;
use super::request_id::request_id_middleware;

pub fn create_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(handlers::health_handler))
        .route("/api/metrics", get(handlers::metrics_handler))
        .route("/api/agents", get(handlers::agents_handler))
        .route("/api/exec", post(handlers::exec_handler))
        .route("/api/audit", get(handlers::audit_handler))
        .route("/api/sandbox", get(handlers::sandbox_handler))
        .route("/api/profile", get(handlers::profile_handler))
        .route("/api/policy/reload", post(handlers::policy_reload_handler))
        .route("/api/budget", get(handlers::budget_handler))
        .route("/api/budget/reset", post(handlers::budget_reset_handler))
        .route("/api/budget/:agent_id", get(handlers::budget_agent_handler))
        .route("/api/agents/:id", delete(handlers::delete_agent_handler))
        .route("/api/agents/:id/reset", post(handlers::reset_agent_handler))
        // Network routes
        .route("/api/network/status", get(handlers::network_status_handler))
        .route("/api/network/block", post(handlers::network_block_handler))
        .route("/api/network/allow", post(handlers::network_allow_handler))
        .route("/api/network/logs", get(handlers::network_logs_handler))
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(cors)
        .with_state(state)
}
