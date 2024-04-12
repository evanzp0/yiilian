use axum::{http::StatusCode, response::{IntoResponse, Response}};
use anyhow::anyhow;

#[derive(Debug)]
pub struct WebError {
    code: StatusCode,
    error: anyhow::Error,
}

impl WebError {
    pub fn new(code: StatusCode, error: anyhow::Error) -> Self {
        WebError { code, error }
    }

    pub fn from_error(error: anyhow::Error) -> Self {
        WebError { code: StatusCode::INTERNAL_SERVER_ERROR, error }
    }

    pub fn from_code(code: StatusCode) -> Self {
        WebError { code, error: anyhow!("") }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        (
            self.code,
            format!("Error: {}", self.error),
        )
        .into_response()
    }
}

impl<E> From<E> for WebError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        WebError::from_error(err.into())
    }
}