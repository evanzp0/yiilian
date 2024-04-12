use std::sync::Arc;

use axum::{extract::MatchedPath, http::Request, routing::get, Router};

use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::trace;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use yiilian_core::common::working_dir::WorkingDir;
use yiilian_web::{common::AppState, STATIC_DIR};

#[tokio::main]
async fn main() {
    let working_dir = WorkingDir::new();
    setup_tracing(&working_dir);

    // dir: web/static
    let static_path = working_dir
        .get_path_by_entry("web")
        .and_then(|p| Some(p.join(STATIC_DIR)))
        .unwrap();
    // file: web/static/404.html
    let file_404 = static_path.clone().join("404.html");

    let app_state = Arc::new(AppState::new(working_dir));
    let serve_dir = ServeDir::new(static_path).not_found_service(ServeFile::new(file_404.clone()));

    let app = Router::new()
        .route("/", get(|| async { trace!("hello") }))
        .nest_service("/static", serve_dir.clone())
        .fallback_service(ServeFile::new(file_404))
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
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn setup_tracing(wd: &WorkingDir) {
    let env_path = wd.get_path_by_entry(".env").unwrap();
    dotenv::from_path(env_path.as_path()).unwrap();

    tracing_subscriber::registry()
        .with(fmt::layer().with_ansi(false))
        .with(EnvFilter::from_env("RUST_LOG"))
        .init();
}
