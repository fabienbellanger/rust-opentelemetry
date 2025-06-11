//! Prometheus metrics layer

use axum::body::Body;
use axum::{extract::MatchedPath, middleware::Next, response::IntoResponse};
use hyper::Request;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use core::fmt;
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
        let system_metrics = SystemMetrics::new("/").await;
        println!("System metrics:\n{}", system_metrics);
        println!("----------------------------------------------------------------");

        // Gauges
        let gauge = gauge!("system_cpu_usage", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.cpu_usage);
        let gauge = gauge!("system_total_memory", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.total_memory as f64);
        let gauge = gauge!("system_used_memory", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.used_memory as f64);
        let gauge = gauge!("system_total_swap", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.total_swap as f64);
        let gauge = gauge!("system_used_swap", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.used_swap as f64);
        let gauge = gauge!("system_total_disks_space", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.total_disks_space as f64);
        let gauge = gauge!("system_used_disks_usage", "service" => "rust-open-telemetry");
        gauge.set(system_metrics.used_disks_space as f64);

        response
    }
}

#[derive(Debug, Clone)]
struct SystemMetrics {
    /// Average CPU usage in percent
    cpu_usage: f32,

    /// Total memory in bytes
    total_memory: u64,

    /// Used memory in bytes
    used_memory: u64,

    /// Total swap space in bytes
    total_swap: u64,

    /// Used swap space in bytes
    used_swap: u64,

    /// Total disk space in bytes for a specified mount point
    total_disks_space: u64,

    /// Used disk space in bytes for a specified mount point
    used_disks_space: u64,
}

impl SystemMetrics {
    async fn new(disk_mount_point: &str) -> Self {
        let mut sys = System::new_all();

        // CPU
        sys.refresh_cpu_usage();
        let mut cpu_usage = sys.global_cpu_usage();
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_cpu_usage();
        cpu_usage += sys.global_cpu_usage();
        cpu_usage /= 2.0;

        // Memory
        sys.refresh_memory();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();

        // Swap
        let total_swap = sys.total_swap();
        let used_swap = sys.used_swap();
        
        // Disks
        let disks = Disks::new_with_refreshed_list();
        let mut total_disks_space = 0;
        let mut used_disks_space = 0;
        for disk in &disks {
            if disk.mount_point() == Path::new(disk_mount_point) {
                total_disks_space += disk.total_space();
                used_disks_space += disk.total_space() - disk.available_space();
            }
        }

        Self {
            cpu_usage: cpu_usage,
            total_memory,
            used_memory,
            total_swap,
            used_swap,
            total_disks_space,
            used_disks_space,
        }
    }
}

impl fmt::Display for SystemMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CPUs:       {:.1}%\n\
             Memory:     {} / {}\n\
             Swap:       {} / {}\n\
             Disk usage: {} / {}",
            self.cpu_usage,
            ByteSize::b(self.used_memory),
            ByteSize::b(self.total_memory),
            ByteSize::b(self.used_swap),
            ByteSize::b(self.total_swap),
            ByteSize::b(self.used_disks_space),
            ByteSize::b(self.total_disks_space),
        )
    }
}
