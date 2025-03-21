use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::instrument;

#[macro_use]
extern crate tracing;

#[instrument(name = "main_function")]
#[tokio::main]
async fn main() {
    env_logger::init();

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    info!("Starting server on 0.0.0.0:3333...");
    let listener = TcpListener::bind("0.0.0.0:3333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
