//! # Monitoring
//!
//! This module provides monitoring infrastructure for health checks, metrics,
//! and distributed tracing. It is intentionally framework-neutral: the SDK keeps
//! in-process registries and tracing spans, while applications choose how to
//! export them.

pub mod health;
pub mod health_server;
pub mod metrics;
pub mod prometheus;
pub mod tracing;

use once_cell::sync::Lazy;
use std::sync::Arc;

/// Output format for the tracing/logging subscriber.
///
/// [`LogFormat::Text`] is the default and preserves the historical human-readable
/// output. [`LogFormat::Json`] emits one structured JSON object per event/span,
/// which is what log aggregators (Loki, ELK, CloudWatch, ...) expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
	/// Human-readable text formatter (default, backward-compatible).
	#[default]
	Text,
	/// Structured JSON formatter.
	Json,
}

impl LogFormat {
	/// Parse a [`LogFormat`] from a case-insensitive string.
	///
	/// Unknown values fall back to [`LogFormat::Text`] so that misconfiguration
	/// never prevents the SDK from initializing.
	#[must_use]
	pub fn parse(value: &str) -> Self {
		match value.trim().to_ascii_lowercase().as_str() {
			"json" => Self::Json,
			_ => Self::Text,
		}
	}
}

impl std::fmt::Display for LogFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Text => f.write_str("text"),
			Self::Json => f.write_str("json"),
		}
	}
}

/// Global monitoring configuration
pub static MONITORING: Lazy<Arc<MonitoringConfig>> =
	Lazy::new(|| Arc::new(MonitoringConfig::from_env()));

/// Monitoring configuration
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MonitoringConfig {
	pub metrics_enabled: bool,
	pub metrics_port: u16,
	pub tracing_enabled: bool,
	pub tracing_endpoint: String,
	pub log_level: String,
	/// Output format used when initializing the tracing subscriber.
	pub log_format: LogFormat,
	pub health_check_enabled: bool,
	pub health_check_port: u16,
}

impl MonitoringConfig {
	/// Creates a new builder for the configuration
	pub fn builder() -> MonitoringConfigBuilder {
		MonitoringConfigBuilder::default()
	}
}

/// Builder for `MonitoringConfig`
#[derive(Debug, Default, Clone)]
pub struct MonitoringConfigBuilder {
	metrics_enabled: Option<bool>,
	metrics_port: Option<u16>,
	tracing_enabled: Option<bool>,
	tracing_endpoint: Option<String>,
	log_level: Option<String>,
	log_format: Option<LogFormat>,
	health_check_enabled: Option<bool>,
	health_check_port: Option<u16>,
}

impl MonitoringConfigBuilder {
	#[must_use]
	pub fn metrics_enabled(mut self, val: bool) -> Self {
		self.metrics_enabled = Some(val);
		self
	}

	#[must_use]
	pub fn metrics_port(mut self, val: u16) -> Self {
		self.metrics_port = Some(val);
		self
	}

	#[must_use]
	pub fn tracing_enabled(mut self, val: bool) -> Self {
		self.tracing_enabled = Some(val);
		self
	}

	#[must_use]
	pub fn tracing_endpoint(mut self, val: String) -> Self {
		self.tracing_endpoint = Some(val);
		self
	}

	#[must_use]
	pub fn log_level(mut self, val: String) -> Self {
		self.log_level = Some(val);
		self
	}

	#[must_use]
	pub fn log_format(mut self, val: LogFormat) -> Self {
		self.log_format = Some(val);
		self
	}

	#[must_use]
	pub fn health_check_enabled(mut self, val: bool) -> Self {
		self.health_check_enabled = Some(val);
		self
	}

	#[must_use]
	pub fn health_check_port(mut self, val: u16) -> Self {
		self.health_check_port = Some(val);
		self
	}

	/// Builds the `MonitoringConfig`. Missing fields fallback to defaults or environment variables.
	pub fn build(self) -> MonitoringConfig {
		let env_default = MonitoringConfig::from_env();
		MonitoringConfig {
			metrics_enabled: self.metrics_enabled.unwrap_or(env_default.metrics_enabled),
			metrics_port: self.metrics_port.unwrap_or(env_default.metrics_port),
			tracing_enabled: self.tracing_enabled.unwrap_or(env_default.tracing_enabled),
			tracing_endpoint: self.tracing_endpoint.unwrap_or(env_default.tracing_endpoint),
			log_level: self.log_level.unwrap_or(env_default.log_level),
			log_format: self.log_format.unwrap_or(env_default.log_format),
			health_check_enabled: self
				.health_check_enabled
				.unwrap_or(env_default.health_check_enabled),
			health_check_port: self.health_check_port.unwrap_or(env_default.health_check_port),
		}
	}
}

impl MonitoringConfig {
	/// Create configuration from environment variables
	pub fn from_env() -> Self {
		Self {
			metrics_enabled: std::env::var("NEO_METRICS_ENABLED")
				.unwrap_or_else(|_| "true".to_string())
				.parse()
				.unwrap_or(true),
			metrics_port: std::env::var("NEO_METRICS_PORT")
				.unwrap_or_else(|_| "9090".to_string())
				.parse()
				.unwrap_or(9090),
			tracing_enabled: std::env::var("NEO_TRACING_ENABLED")
				.unwrap_or_else(|_| "true".to_string())
				.parse()
				.unwrap_or(true),
			tracing_endpoint: std::env::var("NEO_TRACING_ENDPOINT")
				.unwrap_or_else(|_| "http://localhost:4317".to_string()),
			log_level: std::env::var("NEO_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
			log_format: std::env::var("NEO_LOG_FORMAT")
				.map(|v| LogFormat::parse(&v))
				.unwrap_or_default(),
			health_check_enabled: std::env::var("NEO_HEALTH_CHECK_ENABLED")
				.unwrap_or_else(|_| "true".to_string())
				.parse()
				.unwrap_or(true),
			health_check_port: std::env::var("NEO_HEALTH_CHECK_PORT")
				.unwrap_or_else(|_| "8080".to_string())
				.parse()
				.unwrap_or(8080),
		}
	}
}

/// Initialize monitoring subsystems.
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
	if MONITORING.metrics_enabled {
		metrics::init(MONITORING.metrics_port)?;
	}

	#[cfg(feature = "metrics-prometheus")]
	{
		use crate::monitoring::prometheus;
		if MONITORING.metrics_enabled {
			prometheus::init(MONITORING.metrics_port)?;
		}
	}

	if MONITORING.tracing_enabled {
		tracing::init_with_config(
			&MONITORING.tracing_endpoint,
			&MONITORING.log_level,
			MONITORING.log_format,
		)?;
	}

	if MONITORING.health_check_enabled {
		health::init(MONITORING.health_check_port)?;
	}

	#[cfg(feature = "metrics-prometheus")]
	health_server::init(MONITORING.health_check_port)?;

	Ok(())
}

/// Shutdown monitoring subsystems.
pub fn shutdown() {
	metrics::shutdown();
	#[cfg(feature = "metrics-prometheus")]
	prometheus::shutdown();
	tracing::shutdown();
	health::shutdown();
	#[cfg(feature = "metrics-prometheus")]
	health_server::shutdown();
}
