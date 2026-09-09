//! # Prometheus Metrics Adapter
//!
//! Exports the SDK's in-process metrics registry in the Prometheus text
//! exposition format, optionally serving them over an HTTP `/metrics` endpoint
//! that a Prometheus server can scrape.
//!
//! This module is **purely additive** and gated behind the `metrics-prometheus`
//! feature so the default build stays lean:
//!
//! ```toml
//! [dependencies]
//! neo3 = { version = "3.3", features = ["metrics-prometheus"] }
//! ```
//!
//! ## Capabilities
//!
//! * Converts SDK counters / gauges / histograms to Prometheus metrics.
//! * Serves `GET /metrics` for scraping (async, powered by the existing tokio
//!   runtime — no extra HTTP framework dependency).
//! * Filters exported series by `namespace` or `component` query parameters,
//!   e.g. `GET /metrics?component=rpc`.
//! * Optional bearer-token authentication (`Authorization: Bearer <token>`).
//!
//! ## Configuration (environment variables)
//!
//! | Variable | Default | Meaning |
//! |----------|---------|---------|
//! | `NEO_PROMETHEUS_ENABLED` | `true` | Enable the adapter |
//! | `NEO_PROMETHEUS_PORT` | `9090` | HTTP listen port |
//! | `NEO_PROMETHEUS_AUTH_TOKEN` | _unset_ | Require this bearer token |
//! | `NEO_METRICS_NAMESPACE` | _unset_ | Prefix prepended to every metric name |
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(feature = "metrics-prometheus")]
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use neo3::monitoring::{metrics, prometheus};
//!
//! metrics::init(0)?;
//! metrics::increment_counter("rpc.mainnet.getblock.success", 1.0);
//!
//! // Start the scrape endpoint on :9090 (spawns a background task).
//! prometheus::serve(prometheus::PrometheusConfig::from_env()).await?;
//! # Ok(())
//! # }
//! ```

use crate::monitoring::metrics::{snapshot, MetricsSnapshot};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

/// Runtime configuration for the Prometheus adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
	/// TCP port for the `/metrics` HTTP endpoint.
	pub port: u16,
	/// Optional bearer token required to scrape metrics.
	pub auth_token: Option<String>,
	/// Optional prefix prepended to every exported metric name.
	pub namespace: String,
	/// Whether the adapter is enabled.
	pub enabled: bool,
}

impl Default for PrometheusConfig {
	fn default() -> Self {
		Self { port: 9090, auth_token: None, namespace: String::new(), enabled: true }
	}
}

impl PrometheusConfig {
	/// Build a configuration from environment variables, falling back to defaults.
	#[must_use]
	pub fn from_env() -> Self {
		let default = Self::default();
		Self {
			port: std::env::var("NEO_PROMETHEUS_PORT")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(default.port),
			auth_token: std::env::var("NEO_PROMETHEUS_AUTH_TOKEN").ok().filter(|t| !t.is_empty()),
			namespace: std::env::var("NEO_METRICS_NAMESPACE").unwrap_or_default(),
			enabled: std::env::var("NEO_PROMETHEUS_ENABLED")
				.ok()
				.and_then(|v| v.parse().ok())
				.unwrap_or(default.enabled),
		}
	}
}

static CONFIG: OnceCell<PrometheusConfig> = OnceCell::new();

/// Store the adapter configuration so helpers (auth, namespace) can access it.
///
/// Called by [`init`] and [`serve`]; safe to call more than once (later calls
/// are ignored, matching the other monitoring subsystems).
fn store_config(config: PrometheusConfig) {
	let _ = CONFIG.set(config);
}

/// Return the active configuration, or the environment-derived default.
#[must_use]
pub fn config() -> PrometheusConfig {
	CONFIG.get().cloned().unwrap_or_else(PrometheusConfig::from_env)
}

/// Initialize the Prometheus adapter configuration for `port`.
///
/// This registers configuration only; call [`serve`] from an async context to
/// start the scrape endpoint. Kept separate so `monitoring::init` (which is
/// synchronous) can wire configuration without requiring a runtime.
pub fn init(port: u16) -> Result<(), Box<dyn std::error::Error>> {
	let mut config = PrometheusConfig::from_env();
	config.port = port;
	config.enabled = true;
	store_config(config);
	tracing::info!(port, "Prometheus adapter configured");
	Ok(())
}

/// Shutdown hook (metrics are drained by [`crate::monitoring::metrics::shutdown`]).
pub fn shutdown() {
	tracing::debug!("Prometheus adapter shutdown requested");
}

/// Sanitize an SDK metric name into a valid Prometheus identifier.
///
/// Prometheus names must match `[a-zA-Z_:][a-zA-Z0-9_:]*`; SDK names use `.`
/// separators, so every invalid character is replaced with `_`.
fn sanitize(name: &str) -> String {
	let mut out = String::with_capacity(name.len());
	for ch in name.chars() {
		if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
			out.push(ch);
		} else {
			out.push('_');
		}
	}
	// A leading digit is invalid in Prometheus; prefix with a single underscore.
	if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
		out.insert(0, '_');
	}
	// Cap length defensively. `out` is pure ASCII, so 255 is a char boundary.
	out.truncate(255);
	// Guarantee a non-empty, valid identifier.
	if out.is_empty() {
		out.push_str("_empty");
	}
	out
}

/// Return true when `metric_name` passes the namespace/component filters.
fn passes_filter(
	metric_name: &str,
	namespace_filter: Option<&str>,
	component_filter: Option<&str>,
) -> bool {
	// Compare in the sanitized namespace so filters match the names actually
	// emitted, and so untrusted query-param values are sanitized before use.
	let sanitized = sanitize(metric_name);
	if let Some(ns) = namespace_filter {
		if !ns.is_empty() && !sanitized.starts_with(&sanitize(ns)) {
			return false;
		}
	}
	if let Some(component) = component_filter {
		if !component.is_empty() && !sanitized.contains(&sanitize(component)) {
			return false;
		}
	}
	true
}

/// Apply the configured namespace prefix to a sanitized metric name.
fn with_namespace(name: &str) -> String {
	let ns = config().namespace;
	if ns.is_empty() {
		name.to_string()
	} else {
		format!("{}_{}", sanitize(&ns), name)
	}
}

/// Render a metrics snapshot into the Prometheus text exposition format.
///
/// `namespace_filter` keeps only metrics whose name starts with the given
/// prefix; `component_filter` keeps only metrics whose name contains the given
/// substring. Both are optional and independent.
#[must_use]
pub fn render(
	snap: &MetricsSnapshot,
	namespace_filter: Option<&str>,
	component_filter: Option<&str>,
) -> String {
	let mut out = String::new();

	out.push_str("# HELP neo_sdk_up 1 if the Neo SDK metrics exporter is running.\n");
	out.push_str("# TYPE neo_sdk_up gauge\n");
	out.push_str("neo_sdk_up 1\n");

	for (name, value) in &snap.counters {
		if !passes_filter(name, namespace_filter, component_filter) {
			continue;
		}
		let metric = with_namespace(&sanitize(name));
		out.push_str(&format!("# TYPE {metric} counter\n"));
		out.push_str(&format!("{metric} {value}\n"));
	}

	for (name, value) in &snap.gauges {
		if !passes_filter(name, namespace_filter, component_filter) {
			continue;
		}
		let metric = with_namespace(&sanitize(name));
		out.push_str(&format!("# TYPE {metric} gauge\n"));
		out.push_str(&format!("{metric} {value}\n"));
	}

	for (name, values) in &snap.histograms {
		if !passes_filter(name, namespace_filter, component_filter) {
			continue;
		}
		let metric = with_namespace(&sanitize(name));
		let sum: f64 = values.iter().sum();
		let count = values.len();
		out.push_str(&format!("# TYPE {metric} summary\n"));
		out.push_str(&format!("{metric}_sum {sum}\n"));
		out.push_str(&format!("{metric}_count {count}\n"));
	}

	out
}

/// Convenience wrapper: render the current registry snapshot.
#[must_use]
pub fn render_current(namespace_filter: Option<&str>, component_filter: Option<&str>) -> String {
	render(&snapshot(), namespace_filter, component_filter)
}

/// Verify a request's `Authorization` header against the configured token.
///
/// Returns `true` when no token is configured (open scraping) or when the
/// supplied header exactly matches `Bearer <token>`.
#[must_use]
pub fn is_authorized(auth_header: Option<&str>) -> bool {
	match config().auth_token {
		None => true,
		Some(token) => match auth_header {
			// Constant-time comparison of the full `Bearer <token>` header value
			// avoids leaking the token through response-timing side channels.
			// `verify_slices_are_equal` also returns `Err` on length mismatch.
			Some(header) => {
				let expected = format!("Bearer {token}");
				ring::constant_time::verify_slices_are_equal(
					header.as_bytes(),
					expected.as_bytes(),
				)
				.is_ok()
			},
			None => false,
		},
	}
}

/// Parse the `key=value&...` query string of a `/metrics` request.
///
/// Returns `(namespace, component)` filter values when present.
fn parse_query(query: &str) -> (Option<String>, Option<String>) {
	let mut namespace = None;
	let mut component = None;
	for pair in query.split('&') {
		let mut kv = pair.splitn(2, '=');
		match (kv.next(), kv.next()) {
			(Some("namespace"), Some(v)) if !v.is_empty() => namespace = Some(v.to_string()),
			(Some("component"), Some(v)) if !v.is_empty() => component = Some(v.to_string()),
			_ => {},
		}
	}
	(namespace, component)
}

/// Start the `/metrics` HTTP endpoint on the configured port.
///
/// Spawns a background tokio task that accepts connections and serves the
/// Prometheus exposition format. Returns immediately after binding the socket.
#[cfg(feature = "metrics-prometheus")]
pub async fn serve(config: PrometheusConfig) -> Result<(), Box<dyn std::error::Error>> {
	use tokio::net::TcpListener;

	let port = config.port;
	store_config(config);

	let listener = TcpListener::bind(("0.0.0.0", port)).await?;
	tracing::info!(port, "Prometheus /metrics endpoint listening");

	tokio::spawn(async move {
		loop {
			match listener.accept().await {
				Ok((stream, _peer)) => {
					tokio::spawn(async move {
						if let Err(e) = handle_connection(stream).await {
							tracing::debug!(error = %e, "Prometheus connection error");
						}
					});
				},
				Err(e) => {
					tracing::warn!(error = %e, "Prometheus accept error");
				},
			}
		}
	});

	Ok(())
}

/// Handle a single HTTP connection: parse the request line + headers, then
/// reply with metrics or an error status.
#[cfg(feature = "metrics-prometheus")]
async fn handle_connection(
	mut stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	let mut buf = vec![0u8; 8192];
	let n = stream.read(&mut buf).await?;
	let request = String::from_utf8_lossy(&buf[..n]);

	let (status, body, content_type) = route(&request);

	let response = format!(
		"HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
		len = body.len(),
	);
	stream.write_all(response.as_bytes()).await?;
	stream.flush().await?;
	Ok(())
}

/// Route a raw HTTP request to a `(status_line, body, content_type)` tuple.
///
/// Extracted from the connection handler so it can be unit-tested without a
/// real socket.
#[cfg(feature = "metrics-prometheus")]
fn route(request: &str) -> (&'static str, String, &'static str) {
	let mut lines = request.lines();
	let request_line = lines.next().unwrap_or("");
	let mut parts = request_line.split_whitespace();
	let method = parts.next().unwrap_or("");
	let target = parts.next().unwrap_or("");

	let auth_header = request
		.lines()
		.find_map(|l| l.strip_prefix("Authorization: ").or_else(|| l.strip_prefix("authorization: ")));

	if method != "GET" {
		return ("405 Method Not Allowed", "method not allowed\n".to_string(), "text/plain");
	}

	let (path, query) = match target.split_once('?') {
		Some((p, q)) => (p, q),
		None => (target, ""),
	};

	if path != "/metrics" {
		return ("404 Not Found", "not found\n".to_string(), "text/plain");
	}

	if !is_authorized(auth_header) {
		return ("401 Unauthorized", "unauthorized\n".to_string(), "text/plain");
	}

	let (namespace, component) = parse_query(query);
	let body = render_current(namespace.as_deref(), component.as_deref());
	("200 OK", body, "text/plain; version=0.0.4")
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::monitoring::metrics::MetricsSnapshot;
	use std::collections::HashMap;

	fn sample_snapshot() -> MetricsSnapshot {
		let mut counters = HashMap::new();
		counters.insert("rpc.mainnet.getblock.success".to_string(), 12.0);
		counters.insert("transactions.mainnet.transfer.success".to_string(), 3.0);
		let mut gauges = HashMap::new();
		gauges.insert("blockchain.mainnet.height".to_string(), 42.0);
		let mut histograms = HashMap::new();
		histograms.insert("rpc.mainnet.getblock.duration_seconds".to_string(), vec![0.1, 0.3, 0.2]);
		MetricsSnapshot { counters, gauges, histograms }
	}

	#[test]
	fn sanitize_replaces_invalid_characters() {
		assert_eq!(sanitize("rpc.mainnet.getblock"), "rpc_mainnet_getblock");
		assert_eq!(sanitize("a-b/c"), "a_b_c");
		assert_eq!(sanitize("valid_name:foo"), "valid_name:foo");
	}

	#[test]
	fn sanitize_prefixes_leading_digit() {
		assert_eq!(sanitize("1abc"), "_1abc");
	}

	#[test]
	fn render_emits_all_metric_families() {
		let snap = sample_snapshot();
		let out = render(&snap, None, None);
		assert!(out.contains("neo_sdk_up 1"));
		assert!(out.contains("rpc_mainnet_getblock_success 12"));
		assert!(out.contains("# TYPE rpc_mainnet_getblock_success counter"));
		assert!(out.contains("blockchain_mainnet_height 42"));
		assert!(out.contains("# TYPE blockchain_mainnet_height gauge"));
		assert!(out.contains("rpc_mainnet_getblock_duration_seconds_sum"));
		assert!(out.contains("rpc_mainnet_getblock_duration_seconds_count 3"));
	}

	#[test]
	fn render_histogram_sum_is_accurate() {
		let snap = sample_snapshot();
		let out = render(&snap, None, None);
		// 0.1 + 0.3 + 0.2 = 0.6 (allow float formatting)
		let line = out
			.lines()
			.find(|l| l.starts_with("rpc_mainnet_getblock_duration_seconds_sum"))
			.expect("sum line present");
		let value: f64 = line.split_whitespace().last().unwrap().parse().unwrap();
		assert!((value - 0.6).abs() < 1e-9, "sum was {value}");
	}

	#[test]
	fn namespace_filter_keeps_only_prefixed_metrics() {
		let snap = sample_snapshot();
		let out = render(&snap, Some("rpc"), None);
		assert!(out.contains("rpc_mainnet_getblock_success"));
		assert!(!out.contains("transactions_mainnet_transfer_success"));
		assert!(!out.contains("blockchain_mainnet_height"));
	}

	#[test]
	fn component_filter_keeps_only_matching_substring() {
		let snap = sample_snapshot();
		let out = render(&snap, None, Some("transfer"));
		assert!(out.contains("transactions_mainnet_transfer_success"));
		assert!(!out.contains("rpc_mainnet_getblock_success"));
	}

	#[test]
	fn passes_filter_combines_namespace_and_component() {
		assert!(passes_filter("rpc.mainnet.getblock", Some("rpc"), Some("getblock")));
		assert!(!passes_filter("rpc.mainnet.getblock", Some("rpc"), Some("transfer")));
		assert!(!passes_filter("rpc.mainnet.getblock", Some("db"), None));
		assert!(passes_filter("anything", None, None));
	}

	#[test]
	fn parse_query_extracts_filters() {
		assert_eq!(
			parse_query("namespace=rpc&component=getblock"),
			(Some("rpc".to_string()), Some("getblock".to_string()))
		);
		assert_eq!(parse_query(""), (None, None));
		assert_eq!(parse_query("component=x"), (None, Some("x".to_string())));
	}

	#[test]
	fn config_from_env_defaults_when_unset() {
		// Ensure a clean environment for the fields we read.
		std::env::remove_var("NEO_PROMETHEUS_PORT");
		std::env::remove_var("NEO_PROMETHEUS_AUTH_TOKEN");
		let cfg = PrometheusConfig::from_env();
		assert_eq!(cfg.port, 9090);
		assert!(cfg.auth_token.is_none());
	}

	#[test]
	fn authorization_open_when_no_token() {
		// CONFIG may or may not be set by other tests; construct explicitly.
		let cfg = PrometheusConfig { auth_token: None, ..PrometheusConfig::default() };
		store_config(cfg);
		// When no token is configured any header (or none) is accepted.
		assert!(is_authorized(None));
		assert!(is_authorized(Some("Bearer whatever")));
	}
}
