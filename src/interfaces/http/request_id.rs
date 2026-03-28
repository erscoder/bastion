use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Middleware that ensures every request and response carries an `X-Request-ID` header.
///
/// - If the incoming request already has `X-Request-ID`, that value is reused.
/// - Otherwise, a new UUID v4 is generated.
/// - The value is always forwarded in the outgoing response header.
pub async fn request_id_middleware(request: Request<Body>, next: Next) -> Response {
    // Extract existing X-Request-ID or generate a fresh one.
    let request_id = request
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Run the inner handler.
    let mut response = next.run(request).await;

    // Attach the request-id to the response.
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID.clone(), value);
    }

    response
}
