//! # Health Check HTTP Server
//!
//! Serves Kubernetes-style liveness and readiness probes over HTTP:
//!
//! * `GET /healthz` — **liveness**: is the process alive? Returns `200` while
//!   the process is running and the aggregated health is not `Unhealthy`.
//! * `GET /readyz` — **readiness**: can the process serve traffic? Returns `200`
//!   only when every registered check is `Healthy` and resource thresholds
//!   (memory) are within limits.
//! * `GET /status` — detailed JSON snapshot (all checks, memory, connections,
//!   uptime) for dashboards and debugging.
//!
//! The endpoint reuses the SDK's existing in-process [`health`](super::health)
//! registry and the ambient tokio runtime, so it adds no HTTP-framework
//! dependency. It is gated behind the `metrics-prometheus` feature alongside the
//! other HTTP exporters.
//!
//! ## Configuration (environment variables)
//!
//! | Variable | Default | Meaning |
//! |----------|---------|---------|
//! | `NEO_HEALTH_CHECK_PORT` | `8080` | HTTP listen port |
//! | `NEO_HEALTH_MEMORY_DEGRADED_PCT` | `70` | ≥ this memory% ⇒ degraded |
//! | `NEO_HEALTH_MEMORY_UNHEALTHY_PCT` | `85` | ≥ this memory% ⇒ readiness fails |
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(feature = "metrics-prometheus")]
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use neo3::monitoring::health_server::{self, HealthServerConfig};
//!
//! health_server::serve(HealthServerConfig::from_env()).await?;
//! # Ok(())
//! # }
//! ```

use crate::monitoring::health::{self, HealthStatus};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Configuration for the health check HTTP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthServerConfig {
	/// TCP port to listen on.
	pub port: u16,
	/// Memory usage percentage at or above which the service is reported degraded.
	pub memory_degraded_pct: u64,
	/// Memory usage percentage at or above which readiness probes fail.
	pub memory_unhealthy_pct: u64,
}

impl Default for HealthServerConfig {
	fn default() -> Self {
		Self { port: 8080, memory_degraded_pct: 70, memory_unhealthy_pct: 85 }
	}
}

impl HealthServerConfig {
	/// Build configuration from environment variables, falling back to defaults.
	#[must_use]
	pub fn from_env() -> Self {
		let default = Self::default();
		Self {
			port: std::env::var("NEO_HEALTH_CHECK_PORT")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(default.port),
			memory_degraded_pct: std::env::var("NEO_HEALTH_MEMORY_DEGRADED_PCT")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(default.memory_degraded_pct),
			memory_unhealthy_pct: std::env::var("NEO_HEALTH_MEMORY_UNHEALTHY_PCT")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(default.memory_unhealthy_pct),
		}
	}
}

static CONFIG: OnceCell<HealthServerConfig> = OnceCell::new();
static START_TIME: OnceCell<Instant> = OnceCell::new();
static ACTIVE_CONNECTIONS: AtomicU64 = AtomicU64::new(0);

/// Return the active configuration, or the environment-derived default.
#[must_use]
pub fn config() -> HealthServerConfig {
	CONFIG.get().cloned().unwrap_or_else(HealthServerConfig::from_env)
}

/// Initialize the health server configuration for `port`.
///
/// Registers configuration and the process start time so probes can report
/// uptime. Call [`serve`] from an async context to actually bind the socket.
pub fn init(port: u16) -> Result<(), Box<dyn std::error::Error>> {
	let mut config = HealthServerConfig::from_env();
	config.port = port;
	let _ = CONFIG.set(config);
	let _ = START_TIME.set(Instant::now());
	tracing::info!(port, "Health check server configured");
	Ok(())
}

/// Shutdown hook.
pub fn shutdown() {
	tracing::debug!("Health check server shutdown requested");
}

/// Increment the tracked active-connection count.
///
/// Applications call this when a tracked resource (RPC/WebSocket connection,
/// worker, ...) is acquired so `/status` and readiness can report load.
pub fn connection_opened() {
	ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed);
}

/// Decrement the tracked active-connection count (saturating at zero).
pub fn connection_closed() {
	let _ = ACTIVE_CONNECTIONS
		.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |c| Some(c.saturating_sub(1)));
}

/// Current number of tracked active connections.
#[must_use]
pub fn active_connections() -> u64 {
	ACTIVE_CONNECTIONS.load(Ordering::Relaxed)
}

/// Process uptime in seconds since [`init`] was called.
#[must_use]
fn uptime_seconds() -> u64 {
	START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0)
}

fn unix_timestamp() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default()
		.as_secs()
}

fn status_text(status: HealthStatus) -> &'static str {
	match status {
		HealthStatus::Healthy => "healthy",
		HealthStatus::Degraded => "degraded",
		HealthStatus::Unhealthy => "unhealthy",
	}
}

/// Build the liveness probe payload and whether it should return HTTP 200.
///
/// Liveness fails only when aggregated health is `Unhealthy`; a `Degraded`
/// service is still considered alive (matching common k8s conventions).
#[must_use]
pub fn liveness() -> (bool, serde_json::Value) {
	let overall = health::overall_status();
	let alive = overall != HealthStatus::Unhealthy;
	let body = serde_json::json!({
		"status": if alive { "ok" } else { "error" },
		"health": status_text(overall),
		"uptime_seconds": uptime_seconds(),
		"timestamp": unix_timestamp(),
	});
	(alive, body)
}

/// Build the readiness probe payload and whether it should return HTTP 200.
///
/// Readiness requires every registered check to be `Healthy` and memory usage
/// (when observable) to stay below the configured unhealthy threshold.
#[must_use]
pub fn readiness() -> (bool, serde_json::Value) {
	let cfg = config();
	let overall = health::overall_status();
	let checks_ready = overall == HealthStatus::Healthy;

	let memory = health::memory_usage_percent();
	let memory_ready = memory.map(|pct| pct < cfg.memory_unhealthy_pct).unwrap_or(true);

	let ready = checks_ready && memory_ready;

	let check_details: Vec<serde_json::Value> = health::all_checks()
		.into_iter()
		.map(|c| {
			serde_json::json!({
				"name": c.name,
				"status": status_text(c.status),
				"message": c.message,
			})
		})
		.collect();

	let body = serde_json::json!({
		"status": if ready { "ready" } else { "not_ready" },
		"checks": check_details,
		"memory": {
			"usage_percent": memory,
			"degraded_threshold": cfg.memory_degraded_pct,
			"unhealthy_threshold": cfg.memory_unhealthy_pct,
			"ready": memory_ready,
		},
		"active_connections": active_connections(),
		"timestamp": unix_timestamp(),
	});
	(ready, body)
}

/// Build the detailed `/status` payload.
#[must_use]
pub fn status() -> serde_json::Value {
	let overall = health::overall_status();
	let checks: Vec<serde_json::Value> = health::all_checks()
		.into_iter()
		.map(|c| {
			serde_json::json!({
				"name": c.name,
				"status": status_text(c.status),
				"message": c.message,
				"metadata": c.metadata,
			})
		})
		.collect();

	serde_json::json!({
		"status": status_text(overall),
		"version": env!("CARGO_PKG_VERSION"),
		"uptime_seconds": uptime_seconds(),
		"memory_usage_percent": health::memory_usage_percent(),
		"active_connections": active_connections(),
		"checks": checks,
		"timestamp": unix_timestamp(),
	})
}

/// Start the health-check HTTP server on the configured port.
///
/// Spawns a background tokio task and returns once the socket is bound.
#[cfg(feature = "metrics-prometheus")]
pub async fn serve(cfg: HealthServerConfig) -> Result<(), Box<dyn std::error::Error>> {
	use tokio::net::TcpListener;

	let port = cfg.port;
	let _ = CONFIG.set(cfg);
	let _ = START_TIME.set(Instant::now());

	let listener = TcpListener::bind(("0.0.0.0", port)).await?;
	tracing::info!(port, "Health check endpoints listening (/healthz, /readyz, /status)");

	tokio::spawn(async move {
		loop {
			match listener.accept().await {
				Ok((stream, _peer)) => {
					tokio::spawn(async move {
						if let Err(e) = handle_connection(stream).await {
							tracing::debug!(error = %e, "Health server connection error");
						}
					});
				},
				Err(e) => tracing::warn!(error = %e, "Health server accept error"),
			}
		}
	});

	Ok(())
}

#[cfg(feature = "metrics-prometheus")]
async fn handle_connection(
	mut stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	let mut buf = vec![0u8; 4096];
	let n = stream.read(&mut buf).await?;
	let request = String::from_utf8_lossy(&buf[..n]);

	let (status_line, body) = route(&request);
	let response = format!(
		"HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
		len = body.len(),
	);
	stream.write_all(response.as_bytes()).await?;
	stream.flush().await?;
	Ok(())
}

/// Route a raw HTTP request to a `(status_line, json_body)` tuple.
#[cfg(feature = "metrics-prometheus")]
fn route(request: &str) -> (&'static str, String) {
	let request_line = request.lines().next().unwrap_or("");
	let mut parts = request_line.split_whitespace();
	let method = parts.next().unwrap_or("");
	let target = parts.next().unwrap_or("");
	let path = target.split('?').next().unwrap_or("");

	if method != "GET" {
		return ("405 Method Not Allowed", "{\"error\":\"method not allowed\"}".to_string());
	}

	match path {
		"/healthz" | "/health" | "/livez" => {
			let (ok, body) = liveness();
			(if ok { "200 OK" } else { "503 Service Unavailable" }, body.to_string())
		},
		"/readyz" | "/ready" => {
			let (ok, body) = readiness();
			(if ok { "200 OK" } else { "503 Service Unavailable" }, body.to_string())
		},
		"/status" => ("200 OK", status().to_string()),
		_ => ("404 Not Found", "{\"error\":\"not found\"}".to_string()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn config_from_env_defaults() {
		std::env::remove_var("NEO_HEALTH_CHECK_PORT");
		std::env::remove_var("NEO_HEALTH_MEMORY_DEGRADED_PCT");
		std::env::remove_var("NEO_HEALTH_MEMORY_UNHEALTHY_PCT");
		let cfg = HealthServerConfig::from_env();
		assert_eq!(cfg.port, 8080);
		assert_eq!(cfg.memory_degraded_pct, 70);
		assert_eq!(cfg.memory_unhealthy_pct, 85);
	}

	#[test]
	fn liveness_reports_alive_by_default() {
		let (alive, body) = liveness();
		// With no registered checks the registry reports Healthy => alive.
		assert!(alive);
		assert_eq!(body["status"], "ok");
		assert!(body["uptime_seconds"].is_u64());
		assert!(body["timestamp"].is_u64());
	}

	#[test]
	fn readiness_payload_contains_expected_fields() {
		let (_ready, body) = readiness();
		assert!(body["checks"].is_array());
		assert!(body["memory"].is_object());
		assert!(body["memory"]["unhealthy_threshold"].is_u64());
		assert!(body["active_connections"].is_u64());
	}

	#[test]
	fn status_payload_reports_version_and_checks() {
		let body = status();
		assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
		assert!(body["checks"].is_array());
		assert!(body["uptime_seconds"].is_u64());
	}

	#[test]
	fn connection_counter_increments_and_decrements() {
		let start = active_connections();
		connection_opened();
		connection_opened();
		assert_eq!(active_connections(), start + 2);
		connection_closed();
		assert_eq!(active_connections(), start + 1);
		connection_closed();
		assert_eq!(active_connections(), start);
		// Saturates at zero rather than underflowing.
		connection_closed();
		assert_eq!(active_connections(), 0.max(start));
	}

	#[test]
	fn status_text_maps_all_variants() {
		assert_eq!(status_text(HealthStatus::Healthy), "healthy");
		assert_eq!(status_text(HealthStatus::Degraded), "degraded");
		assert_eq!(status_text(HealthStatus::Unhealthy), "unhealthy");
	}
}
