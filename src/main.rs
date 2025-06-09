#[macro_use]
extern crate tracing;
mod prometheus;

use crate::prometheus::PrometheusMetric;
use axum::http::StatusCode;
use axum::{middleware, routing::get, Router};
use opentelemetry::{trace::TracerProvider, KeyValue};
use opentelemetry_otlp::{SpanExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use std::future::ready;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::{instrument, Level};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracer_provider() -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .with_compression(opentelemetry_otlp::Compression::Gzip)
        .with_timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attribute(KeyValue::new("service.name", "rust-open-telemetry"))
                .build(),
        )
        .build()
}

fn init_tracing_subscriber() -> SdkTracerProvider {
    let tracer_provider = init_tracer_provider();

    let tracer = tracer_provider.tracer("rust-open-telemetry");

    tracing_subscriber::registry()
        .with(LevelFilter::from_level(Level::INFO))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    tracer_provider
}

#[tokio::main]
async fn main() {
    // let _guard = init_tracing_subscriber();

    let mut app = Router::new()
        .route("/", get(home))
        .route("/error", get(error))
        .route("/hello", get(hello));

    let prometheus = PrometheusMetric::get_handle().unwrap();
    app = app
        .nest(
            "/metrics",
            Router::new().route("/", get(move || ready(prometheus.render()))),
        )
        .route_layer(middleware::from_fn(PrometheusMetric::get_layer));

    info!("Starting server on 0.0.0.0:3333...");
    let listener = TcpListener::bind("0.0.0.0:3333").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[instrument(name = "home")]
async fn home() -> &'static str {
    info!("Home");

    "Home"
}

#[instrument(name = "error")]
async fn error() -> (StatusCode, &'static str) {
    error!("Home");

    tokio::time::sleep(Duration::from_secs(1)).await;

    (StatusCode::INTERNAL_SERVER_ERROR, "Home")
}

#[instrument(name = "hello")]
async fn hello() -> &'static str {
    tokio::time::sleep(Duration::from_secs(1)).await;

    warn!("Hello, World!");

    get_hello("Fabien")
}

#[instrument(name = "get_hello")]
fn get_hello(s: &str) -> &'static str {
    info!("Hello, {}!", s);

    "Hello, World!"
}
