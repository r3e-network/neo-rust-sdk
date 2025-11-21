#![allow(dead_code, unused_imports)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Metrics configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
	pub enabled: bool,
	pub port: u16,
	pub path: String,
	pub export_interval: Duration,
}

impl Default for MetricsConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			port: 9090,
			path: "/metrics".to_string(),
			export_interval: Duration::from_secs(60),
		}
	}
}

/// Metric type
#[derive(Debug, Clone)]
pub enum MetricType {
	Counter,
	Gauge,
	Histogram,
	Summary,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
	Counter(u64),
	Gauge(f64),
	Histogram(Vec<f64>),
	Summary(SummaryData),
}

/// Summary data for percentile calculations
#[derive(Debug, Clone)]
pub struct SummaryData {
	pub count: u64,
	pub sum: f64,
	pub quantiles: Vec<(f64, f64)>, // (quantile, value)
}

/// Metric definition
#[derive(Debug, Clone)]
pub struct Metric {
	pub name: String,
	pub help: String,
	pub metric_type: MetricType,
	pub labels: HashMap<String, String>,
	pub value: MetricValue,
	pub timestamp: Instant,
}

/// Metrics registry
pub struct MetricsRegistry {
	metrics: Arc<RwLock<HashMap<String, Metric>>>,
}

impl MetricsRegistry {
	pub fn new() -> Self {
		Self { metrics: Arc::new(RwLock::new(HashMap::new())) }
	}

	/// Register a new metric
	pub fn register(&self, metric: Metric) {
		let mut metrics = self.metrics.write().unwrap();
		metrics.insert(metric.name.clone(), metric);
	}

	/// Get a metric
	pub fn get(&self, name: &str) -> Option<Metric> {
		let metrics = self.metrics.read().unwrap();
		metrics.get(name).cloned()
	}

	/// Update a metric value
	pub fn update(&self, name: &str, value: MetricValue) {
		let mut metrics = self.metrics.write().unwrap();
		if let Some(metric) = metrics.get_mut(name) {
			metric.value = value;
			metric.timestamp = Instant::now();
		}
	}

	/// Get all metrics
	pub fn all(&self) -> Vec<Metric> {
		let metrics = self.metrics.read().unwrap();
		metrics.values().cloned().collect()
	}

	/// Export metrics in Prometheus format
	pub fn export_prometheus(&self) -> String {
		let metrics = self.metrics.read().unwrap();
		let mut output = String::new();

		for metric in metrics.values() {
			// Write help text
			output.push_str(&format!("# HELP {} {}\n", metric.name, metric.help));

			// Write type
			let type_str = match metric.metric_type {
				MetricType::Counter => "counter",
				MetricType::Gauge => "gauge",
				MetricType::Histogram => "histogram",
				MetricType::Summary => "summary",
			};
			output.push_str(&format!("# TYPE {} {}\n", metric.name, type_str));

			// Write metric value with labels
			let labels = if metric.labels.is_empty() {
				String::new()
			} else {
				let label_pairs: Vec<String> =
					metric.labels.iter().map(|(k, v)| format!("{}=\"{}\"", k, v)).collect();
				format!("{{{}}}", label_pairs.join(","))
			};
			let label_suffix = if labels.is_empty() {
				String::new()
			} else {
				format!(",{}", &labels[1..labels.len() - 1])
			};

			match &metric.value {
				MetricValue::Counter(v) => {
					output.push_str(&format!("{}{} {}\n", metric.name, labels, v));
				},
				MetricValue::Gauge(v) => {
					output.push_str(&format!("{}{} {}\n", metric.name, labels, v));
				},
				MetricValue::Histogram(values) => {
					// Calculate buckets
					let buckets =
						vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
					let mut counts = vec![0u64; buckets.len()];
					let mut sum = 0.0;

					for value in values {
						sum += value;
						for (i, &bucket) in buckets.iter().enumerate() {
							if *value <= bucket {
								counts[i] += 1;
							}
						}
					}

					for (i, &bucket) in buckets.iter().enumerate() {
						output.push_str(&format!(
							"{}_bucket{{le=\"{}\"{}}} {}\n",
							metric.name, bucket, label_suffix, counts[i]
						));
					}
					output.push_str(&format!(
						"{}_bucket{{le=\"+Inf\"{}}} {}\n",
						metric.name,
						label_suffix,
						values.len()
					));
					output.push_str(&format!("{}_sum{} {}\n", metric.name, labels, sum));
					output.push_str(&format!("{}_count{} {}\n", metric.name, labels, values.len()));
				},
				MetricValue::Summary(data) => {
					for (quantile, value) in &data.quantiles {
						output.push_str(&format!(
							"{}{{quantile=\"{}\"{}}} {}\n",
							metric.name, quantile, label_suffix, value
						));
					}
					output.push_str(&format!("{}_sum{} {}\n", metric.name, labels, data.sum));
					output.push_str(&format!("{}_count{} {}\n", metric.name, labels, data.count));
				},
			}
		}

		output
	}
}

/// Metrics collector
pub struct MetricsCollector {
	registry: Arc<MetricsRegistry>,
	config: MetricsConfig,
	server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MetricsCollector {
	pub fn new(config: MetricsConfig) -> Result<Self, Box<dyn std::error::Error>> {
		let registry = Arc::new(MetricsRegistry::new());

		// Register default metrics
		registry.register(Metric {
			name: "neorust_info".to_string(),
			help: "NeoRust application information".to_string(),
			metric_type: MetricType::Gauge,
			labels: HashMap::from([("version".to_string(), env!("CARGO_PKG_VERSION").to_string())]),
			value: MetricValue::Gauge(1.0),
			timestamp: Instant::now(),
		});

		Ok(Self { registry, config, server_handle: None })
	}

	/// Start metrics HTTP server
	pub fn start_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
		if !self.config.enabled {
			return Ok(());
		}

		let registry = Arc::clone(&self.registry);
		let port = self.config.port;
		let _path = self.config.path.clone();

		let handle = tokio::spawn(async move {
			// In production, use a proper HTTP server like warp or actix-web
			// This is a simplified example
			log::info!("Metrics server started on port {}", port);

			// Simulate metrics server
			loop {
				tokio::time::sleep(Duration::from_secs(60)).await;
				let metrics = registry.export_prometheus();
				log::debug!("Metrics snapshot:\n{}", metrics);
			}
		});

		self.server_handle = Some(handle);
		Ok(())
	}

	/// Stop metrics server
	pub fn stop_server(&mut self) {
		if let Some(handle) = self.server_handle.take() {
			handle.abort();
		}
	}

	/// Record a metric
	pub fn record(&self, name: &str, value: f64, labels: Vec<(&str, &str)>) {
		let label_map: HashMap<String, String> =
			labels.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();

		if let Some(mut metric) = self.registry.get(name) {
			metric.labels = label_map;
			metric.value = MetricValue::Gauge(value);
			metric.timestamp = Instant::now();
			self.registry.update(name, metric.value);
		} else {
			self.registry.register(Metric {
				name: name.to_string(),
				help: format!("Metric {}", name),
				metric_type: MetricType::Gauge,
				labels: label_map,
				value: MetricValue::Gauge(value),
				timestamp: Instant::now(),
			});
		}
	}

	/// Increment a counter
	pub fn increment(&self, name: &str, labels: Vec<(&str, &str)>) {
		let label_map: HashMap<String, String> =
			labels.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();

		if let Some(mut metric) = self.registry.get(name) {
			if let MetricValue::Counter(v) = metric.value {
				metric.value = MetricValue::Counter(v + 1);
				metric.labels = label_map;
				metric.timestamp = Instant::now();
				self.registry.update(name, metric.value);
			}
		} else {
			self.registry.register(Metric {
				name: name.to_string(),
				help: format!("Counter {}", name),
				metric_type: MetricType::Counter,
				labels: label_map,
				value: MetricValue::Counter(1),
				timestamp: Instant::now(),
			});
		}
	}

	/// Update a gauge
	pub fn gauge(&self, name: &str, value: f64, labels: Vec<(&str, &str)>) {
		self.record(name, value, labels);
	}

	/// Record a histogram observation
	pub fn histogram(&self, name: &str, value: f64, labels: Vec<(&str, &str)>) {
		let label_map: HashMap<String, String> =
			labels.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();

		if let Some(mut metric) = self.registry.get(name) {
			if let MetricValue::Histogram(ref mut values) = metric.value {
				values.push(value);
				metric.labels = label_map;
				metric.timestamp = Instant::now();
				self.registry.update(name, metric.value.clone());
			}
		} else {
			self.registry.register(Metric {
				name: name.to_string(),
				help: format!("Histogram {}", name),
				metric_type: MetricType::Histogram,
				labels: label_map,
				value: MetricValue::Histogram(vec![value]),
				timestamp: Instant::now(),
			});
		}
	}

	/// Get metrics in Prometheus format
	pub fn export_prometheus(&self) -> String {
		self.registry.export_prometheus()
	}
}

/// Timer for recording durations
pub struct Timer {
	name: String,
	labels: Vec<(String, String)>,
	start: Instant,
	collector: Arc<MetricsCollector>,
}

impl Timer {
	pub fn new(name: &str, collector: Arc<MetricsCollector>) -> Self {
		Self { name: name.to_string(), labels: Vec::new(), start: Instant::now(), collector }
	}

	pub fn with_label(mut self, key: &str, value: &str) -> Self {
		self.labels.push((key.to_string(), value.to_string()));
		self
	}

	pub fn observe(self) {
		let duration = self.start.elapsed().as_secs_f64();
		let labels: Vec<(&str, &str)> =
			self.labels.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
		self.collector.histogram(&self.name, duration, labels);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_metrics_registry() {
		let registry = MetricsRegistry::new();

		registry.register(Metric {
			name: "test_counter".to_string(),
			help: "Test counter metric".to_string(),
			metric_type: MetricType::Counter,
			labels: HashMap::new(),
			value: MetricValue::Counter(42),
			timestamp: Instant::now(),
		});

		let metric = registry.get("test_counter");
		assert!(metric.is_some());

		if let Some(m) = metric {
			if let MetricValue::Counter(v) = m.value {
				assert_eq!(v, 42);
			}
		}
	}

	#[test]
	fn test_prometheus_export() {
		let registry = MetricsRegistry::new();

		registry.register(Metric {
			name: "test_gauge".to_string(),
			help: "Test gauge metric".to_string(),
			metric_type: MetricType::Gauge,
			labels: HashMap::from([("env".to_string(), "test".to_string())]),
			value: MetricValue::Gauge(3.14),
			timestamp: Instant::now(),
		});

		let export = registry.export_prometheus();
		assert!(export.contains("# HELP test_gauge"));
		assert!(export.contains("# TYPE test_gauge gauge"));
		assert!(export.contains("test_gauge{env=\"test\"}"));
	}
}
