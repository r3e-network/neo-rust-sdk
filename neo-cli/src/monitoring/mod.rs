#![allow(dead_code, unused_imports)]

/// Monitoring module for Neo CLI
/// Provides comprehensive logging, metrics, and monitoring capabilities
pub mod logger;
pub mod metrics;

pub use logger::{
	init_logger, AuditLogger, LogFormat, LoggerConfig, PerformanceLogger, StructuredLogger,
};
pub use metrics::{MetricsCollector, MetricsConfig, MetricsRegistry};

use std::sync::Arc;

/// Initialize monitoring subsystem
pub fn initialize_monitoring(
	logger_config: LoggerConfig,
	metrics_config: MetricsConfig,
) -> Result<MonitoringContext, Box<dyn std::error::Error>> {
	// Initialize logger
	init_logger(logger_config)?;

	// Initialize metrics
	let mut collector = MetricsCollector::new(metrics_config)?;
	// Start metrics server if enabled
	collector.start_server()?;
	let metrics = Arc::new(collector);

	Ok(MonitoringContext { metrics })
}

/// Monitoring context containing all monitoring components
pub struct MonitoringContext {
	pub metrics: Arc<MetricsCollector>,
}

impl MonitoringContext {
	/// Record a metric
	pub fn record_metric(&self, name: &str, value: f64, labels: Vec<(&str, &str)>) {
		self.metrics.record(name, value, labels);
	}

	/// Increment a counter
	pub fn increment_counter(&self, name: &str, labels: Vec<(&str, &str)>) {
		self.metrics.increment(name, labels);
	}

	/// Update a gauge
	pub fn update_gauge(&self, name: &str, value: f64, labels: Vec<(&str, &str)>) {
		self.metrics.gauge(name, value, labels);
	}

	/// Record a histogram observation
	pub fn observe_histogram(&self, name: &str, value: f64, labels: Vec<(&str, &str)>) {
		self.metrics.histogram(name, value, labels);
	}
}
