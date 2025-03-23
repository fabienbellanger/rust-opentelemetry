use std::convert::Infallible;
use std::sync::Arc;
use axum::{Router, routing::get};
use opentelemetry::{KeyValue, trace::TracerProvider, global};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use std::time::Duration;
use axum::body::Body;
use axum::http::{Request};
use axum::middleware::Next;
use axum::response::Response;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{Level, instrument};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, util::SubscriberInitExt};

#[macro_use]
extern crate tracing;

fn _init_tracing() {
    let format = tracing_subscriber::fmt::format()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
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
                .with_attribute(KeyValue::new("service.name", "rust-opentelemetry"))
                .build(),
        )
        .build()
}

fn init_metrics_provider() -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .with_compression(opentelemetry_otlp::Compression::Gzip)
        .with_timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attribute(KeyValue::new("service.name", "rust-opentelemetry"))
                .build(),
        )
        .build()
}

fn init_tracing_subscriber() -> SdkTracerProvider {
    let tracer_provider = init_tracer_provider();

    let tracer = tracer_provider.tracer("rust-opentelemetry");

    tracing_subscriber::registry()
        // The global level filter prevents the exporter network stack
        // from reentering the globally installed OpenTelemetryLayer with
        // its own spans while exporting, as the libraries should not use
        // tracing levels below DEBUG. If the OpenTelemetry layer needs to
        // trace spans and events with higher verbosity levels, consider using
        // per-layer filtering to target the telemetry layer specifically,
        // e.g. by target matching.
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

    tracer_provider
}

struct Metrics {
    request_counter: opentelemetry::metrics::Counter<u64>,
}

impl Metrics {
    fn new(meter: opentelemetry::metrics::Meter) -> Self {
        let request_counter = meter.u64_counter("http_requests_total")
            .with_description("Total number of HTTP requests")
            .build();

        Metrics { request_counter }
    }

    async fn record_request(&self) {
        self.request_counter.add(1, &[]);
    }
}

async fn metrics_middleware(req: Request<Body>, next: Next, metrics: Arc<Mutex<Metrics>>) -> Result<Response, axum::Error> {
    let response = next.run(req).await;
    metrics.lock().await.record_request().await;
    Ok(response)
}

#[tokio::main]
async fn main() {
    let _guard = init_tracing_subscriber();

    let meter_provider = init_metrics_provider();
    global::set_meter_provider(meter_provider);

    let meter = global::meter("rust-opentelemetry");
    let metrics = Arc::new(Mutex::new(Metrics::new(meter)));

    let app = Router::new().route("/", get(hello));
        /*.layer(axum::middleware::from_fn(move |req, next| {
            let metrics = metrics.clone();
            async move {
                metrics_middleware(req, next, metrics).await.map_err(|_| Infallible::from(Ok(())))
            }
        }));*/

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
