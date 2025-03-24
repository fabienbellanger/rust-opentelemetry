mod prometheus;

use std::future::ready;
use axum::{Router, routing::get, middleware};
use opentelemetry::{KeyValue, trace::TracerProvider};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::{Level, instrument};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use crate::prometheus::PrometheusMetric;

#[macro_use]
extern crate tracing;

fn init_tracer_provider() -> SdkTracerProvider {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
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
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    tracer_provider
}

#[tokio::main]
async fn main() {
    let _guard = init_tracing_subscriber();

    let mut app = Router::new().route("/", get(hello));

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

#[instrument(name = "hello")]
async fn hello() -> &'static str {
    warn!("Hello, World!");

    get_hello("Fabien")
}

#[instrument(name = "get_hello")]
fn get_hello(s: &str) -> &'static str {
    info!("Hello, {}!", s);

    "Hello, World!"
}
