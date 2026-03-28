use axum::{body::Body, extract::State, http::Request, middleware::Next, response::IntoResponse};
use super::state::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    if let Some(auth) = auth_header {
        if let Some(encoded) = auth.strip_prefix("Basic ") {
            use base64::Engine;
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(credentials) = String::from_utf8(decoded) {
                    if let Some((user, pass)) = credentials.split_once(':') {
                        if user == state.config.username && pass == state.config.password {
                            return Ok(next.run(request).await);
                        }
                    }
                }
            }
        }
    }

    Err((axum::http::StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
}
