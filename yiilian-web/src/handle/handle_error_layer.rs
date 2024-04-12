use axum::{body::Body, http::{Request, StatusCode}, middleware::Next, response::IntoResponse};
use tracing::error;

pub async fn handler_error_layer(req: Request<Body>, next: Next) -> impl IntoResponse
{
    let response = next.run(req).await;

    if response.status() == StatusCode::INTERNAL_SERVER_ERROR {
        let body = response.into_body();
        let data = axum::body::to_bytes(body, usize::MAX).await.unwrap();

        let data = String::from_utf8_lossy(&data);
        error!("{:?}", data);

        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response();

    }

    response
}