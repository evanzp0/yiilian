use axum::{extract::MatchedPath, http::Request, routing::get, Router};

use tera::Tera;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use yiilian_core::common::working_dir::WorkingDir;
use yiilian_web::{common::{init_app_state, AppState}, handle::root, STATIC_DIR};

#[tokio::main]
async fn main() {
    let working_dir = WorkingDir::new();
    setup_tracing(&working_dir);

    let web_dir = working_dir.get_path_by_entry("web").unwrap();

    let tera = {
        let tpl_wld = web_dir.to_str().unwrap().to_owned() + "/**/*.tpl";
        Tera::new(&tpl_wld).unwrap()
    };
    
    // dir: web/static
    let static_dir = web_dir.join(STATIC_DIR);

    // file: web/static/404.html
    let file_404_path = static_dir.clone().join("404.html");

    // file: web/robots.txt
    let robots_txt = ServeFile::new("./web/robots.txt");

    init_app_state(AppState::new(working_dir, tera));

    let serve_dir = ServeDir::new(static_dir).not_found_service(ServeFile::new(file_404_path.clone()));

    let app = Router::new()
        .route("/", get(root))
        .nest_service("/static", serve_dir.clone())
        .nest_service("/robots.txt", robots_txt)
        .fallback_service(ServeFile::new(file_404_path))
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
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn setup_tracing(wd: &WorkingDir) {
    let env_path = wd.get_path_by_entry(".env").unwrap();
    dotenv::from_path(env_path.as_path()).unwrap();

    tracing_subscriber::registry()
        .with(fmt::layer().with_ansi(true))
        .with(EnvFilter::from_env("RUST_LOG"))
        .init();
}
