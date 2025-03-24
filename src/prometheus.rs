//! Prometheus metrics layer

use axum::body::Body;
use axum::{extract::MatchedPath, middleware::Next, response::IntoResponse};
use hyper::Request;
use metrics::{counter, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

pub const SECONDS_DURATION_BUCKETS: &[f64; 11] = &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

pub struct PrometheusMetric {}

impl PrometheusMetric {
    /// Return a new `PrometheusHandle`
    pub fn get_handle() -> Result<PrometheusHandle, String> {
        PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full("http_requests_duration_seconds".to_string()),
                SECONDS_DURATION_BUCKETS,
            )
            .map_err(|err| err.to_string())?
            .install_recorder()
            .map_err(|err| err.to_string())
    }

    /// Layer tracking requests
    pub async fn get_layer(req: Request<Body>, next: Next) -> impl IntoResponse {
        let start = Instant::now();
        let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
            matched_path.as_str().to_owned()
        } else {
            req.uri().path().to_owned()
        };
        let method = req.method().clone();

        let response = next.run(req).await;

        let latency = start.elapsed().as_secs_f64();
        let status = response.status().as_u16().to_string();
        let labels = [
            ("method", method.to_string()),
            ("path", path),
            ("service", "rust-open-telemetry".to_owned()),
            ("status", status),
        ];

        let counter = counter!("http_requests_total", &labels);
        counter.increment(1);
        let histogram = histogram!("http_requests_duration_seconds", &labels);
        histogram.record(latency);

        response
    }
}
