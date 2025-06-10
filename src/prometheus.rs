//! Prometheus metrics layer

use axum::body::Body;
use axum::{extract::MatchedPath, middleware::Next, response::IntoResponse};
use hyper::Request;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::path::Path;
use std::time::Instant;
use sysinfo::{Disks, System};
use bytesize::ByteSize;

pub const SECONDS_DURATION_BUCKETS: &[f64; 11] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

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
        // HTTP request metrics
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

        // System metrics
        let mut sys = System::new_all();
        
        // CPU
        sys.refresh_cpu_usage();
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_cpu_usage();
        let gauge = gauge!("system_cpu_usage", "service" => "rust-open-telemetry");
        gauge.set(sys.global_cpu_usage());
        println!("      CPUs: {:.0}% - {} cores", sys.global_cpu_usage(), sys.cpus().len());

        // Memory
        sys.refresh_memory();
        let gauge = gauge!("system_total_memory", "service" => "rust-open-telemetry");
        gauge.set(sys.total_memory() as f64);
        let gauge = gauge!("system_used_memory", "service" => "rust-open-telemetry");
        gauge.set(sys.used_memory() as f64);
        println!("    Memory: {} / {} = {:.1}% (available {})",
            ByteSize::b(sys.used_memory()).display().si(), 
            ByteSize::b(sys.total_memory()).display().si(),
            (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0,
            ByteSize::b(sys.total_memory() - sys.used_memory()).display().si());

        // Disks usage
        let disks = Disks::new_with_refreshed_list();
        let mut total_space = 0;
        let mut total_used = 0;
        let mut total_available = 0;
        for disk in &disks {
            // println!("{disk:?}");
            if disk.mount_point() == Path::new("/") {
                total_available += disk.available_space();
                total_space += disk.total_space();
                total_used += disk.total_space() - disk.available_space();
            }
        }
        let gauge = gauge!("system_total_disks_space", "service" => "rust-open-telemetry");
        gauge.set(total_space as f64);
        let gauge = gauge!("system_used_disks_usage", "service" => "rust-open-telemetry");
        gauge.set(total_used as f64);
        println!("Disk usage: {} / {} = {:.1}% (available {})", 
            ByteSize::b(total_used).display().si(), 
            ByteSize::b(total_space).display().si(), 
            (total_used as f64 / total_space as f64) * 100.0, 
            ByteSize::b(total_available).display().si());

        println!("----------------------------------------------------------------");

        response
    }
}
