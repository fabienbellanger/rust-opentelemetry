use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::instrument;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

#[macro_use]
extern crate tracing;

fn init_tracing() {
    let format = tracing_subscriber::fmt::format()
        .with_level(true) // don't include levels in formatted output
        .with_target(true) // don't include targets
        .with_thread_ids(false) // include the thread ID of the current thread
        .with_thread_names(false) // include the name of the current thread
        .with_file(true)
        .with_line_number(true);

    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .event_format(format.pretty())
        .with_writer(std::io::stdout);

    let subscriber = Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(layer);

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let app = Router::new().route("/", get(hello));

    info!("Starting server on 0.0.0.0:3333...");
    let listener = TcpListener::bind("0.0.0.0:3333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[instrument(name = "hello")]
async fn hello() -> &'static str {
    warn!("Hello, World!");

    "Hello, World!"
}
