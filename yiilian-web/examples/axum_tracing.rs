use std::{fmt, str::FromStr};

use axum::{
    extract::{MatchedPath, Query}, http::Request, response::{IntoResponse, Response}, routing::*
};

use serde::{de, Deserialize, Deserializer, Serialize};
use tower_http::trace::TraceLayer;
use tracing::*;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let app = app();

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn app() -> Router {
    
    Router::new()
        .route("/", get(handler))
        .route("/bar", get(bar_handler))
        .layer(
            TraceLayer::new_for_http()
                // Create our own span for the request and include the matched path. The matched
                // path is useful for figuring out which handler the request was routed to.
                .make_span_with(|req: &Request<_>| {
                    let method = req.method();
                    let uri = req.uri();

                    // axum automatically adds this extension.
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                // By default `TraceLayer` will log 5xx responses but we're doing our specific
                // logging of errors so disable that
                .on_failure(()),
        )
}

async fn handler(Query(params): Query<Params>) -> String {
    format!("{params:?}")
}

#[instrument]
async fn bar_handler(Query(params): Query<Params>) -> Result<impl IntoResponse, WebError> {
    // Emit events using the `tracing` macros.
    debug!("Debug message");
    info!("Info message");
    warn!("Warn message");
    error!("Error message");
    event!(name: "exception", Level::ERROR, exception.message = "error message");

    // Create new spans using the `tracing` macros.
    let _span = tracing::info_span!("DB Query");

    if params.bar.is_none() {
        return Err(WebError {
            message: "Error (bar is none)".to_owned(),
        });
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Params {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    foo: Option<i32>,
    bar: Option<String>,
}

/// Serde deserialization decorator to map empty Strings to None,
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct WebError {
    message: String,
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let body = self.message;

        body.into_response()
    }
}
