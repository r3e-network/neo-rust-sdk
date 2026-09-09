//! # Distributed Tracing
//!
//! Structured tracing integration built on the SDK's `tracing` dependency.
//!
//! This module supports:
//!
//! * **Text or JSON log output** — selected via [`LogFormat`]. The default stays
//!   text so existing users and tests are unaffected.
//! * **Correlation / request IDs** — see [`with_correlation_id`], [`correlation_span`]
//!   and [`set_correlation_id`] — so a single logical request can be followed
//!   across every span it touches.
//! * **Real OTLP export** (optional) — when the crate is built with the `otlp`
//!   feature, spans are exported to an OpenTelemetry collector over gRPC using
//!   the configured `tracing_endpoint`. Without the feature the SDK keeps only
//!   the lightweight in-process subscriber.

use super::LogFormat;
use tracing::Level;

/// Field name used to attach a correlation/request ID to spans.
pub const CORRELATION_ID_FIELD: &str = "correlation_id";

/// HTTP header used to propagate a correlation/request ID across process
/// boundaries (e.g. attached to outbound RPC requests). Lowercase to match the
/// canonical HTTP/2 header form.
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// Resolve the OTLP exporter endpoint from configuration and environment.
///
/// Resolution order (first non-empty wins):
/// 1. the explicitly configured `configured` endpoint (from `NEO_TRACING_ENDPOINT`),
/// 2. the OpenTelemetry-standard `OTEL_EXPORTER_OTLP_ENDPOINT` environment variable.
///
/// Returns `None` when neither is set, signalling that OTLP export should be
/// skipped. Kept as a pure function so it can be unit-tested without enabling
/// the heavy `otlp` feature.
#[must_use]
pub fn resolve_otlp_endpoint(configured: &str) -> Option<String> {
	let trimmed = configured.trim();
	if !trimmed.is_empty() {
		return Some(trimmed.to_string());
	}
	std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
		.ok()
		.map(|v| v.trim().to_string())
		.filter(|v| !v.is_empty())
}

/// Parse a log level string into a [`tracing::Level`].
fn parse_level(log_level: &str) -> Level {
	match log_level.to_ascii_lowercase().as_str() {
		"trace" => Level::TRACE,
		"debug" => Level::DEBUG,
		"warn" => Level::WARN,
		"error" => Level::ERROR,
		_ => Level::INFO,
	}
}

/// Initialize the tracing subscriber using the historical text format.
///
/// Retained for backward compatibility; prefer [`init_with_config`].
pub fn init(endpoint: &str, log_level: &str) -> Result<(), Box<dyn std::error::Error>> {
	init_with_config(endpoint, log_level, LogFormat::Text)
}

/// Initialize the tracing subscriber honoring the requested [`LogFormat`].
///
/// When the crate is compiled with the `otlp` feature and `endpoint` is
/// non-empty, an OpenTelemetry OTLP exporter layer is attached in addition to
/// the local formatter. Without the `otlp` feature only the local formatter is
/// installed, keeping the default build lean.
pub fn init_with_config(
	endpoint: &str,
	log_level: &str,
	format: LogFormat,
) -> Result<(), Box<dyn std::error::Error>> {
	let level = parse_level(log_level);

	#[cfg(not(feature = "otlp"))]
	{
		// No exporter is wired without the `otlp` feature; the endpoint is only
		// recorded for observability.
		let _ = endpoint;
		let builder = tracing_subscriber::fmt().with_max_level(level);
		match format {
			LogFormat::Json => builder
				.json()
				.try_init()
				.map_err(|e| format!("Failed to initialize tracing subscriber: {e}"))?,
			LogFormat::Text => builder
				.try_init()
				.map_err(|e| format!("Failed to initialize tracing subscriber: {e}"))?,
		}
	}

	#[cfg(feature = "otlp")]
	{
		use tracing_subscriber::layer::SubscriberExt;
		use tracing_subscriber::util::SubscriberInitExt;
		use tracing_subscriber::Layer;

		let filter = tracing_subscriber::filter::LevelFilter::from_level(level);
		let fmt_layer: Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync> = match format {
			LogFormat::Json => tracing_subscriber::fmt::layer().json().boxed(),
			LogFormat::Text => tracing_subscriber::fmt::layer().boxed(),
		};

		let mut layers: Vec<Box<dyn Layer<tracing_subscriber::Registry> + Send + Sync>> =
			vec![fmt_layer];

		match resolve_otlp_endpoint(endpoint) {
			None => tracing::debug!(
				"OTLP export skipped: no endpoint configured (NEO_TRACING_ENDPOINT / OTEL_EXPORTER_OTLP_ENDPOINT)"
			),
			Some(resolved) => match build_otlp_layer(&resolved) {
				Ok(layer) => layers.push(layer),
				Err(e) => eprintln!("OTLP exporter disabled: {e}"),
			},
		}

		tracing_subscriber::registry()
			.with(layers)
			.with(filter)
			.try_init()
			.map_err(|e| format!("Failed to initialize tracing subscriber: {e}"))?;
	}

	if !endpoint.is_empty() {
		tracing::info!(endpoint = endpoint, format = %format, "Tracing initialized");
	} else {
		tracing::info!(format = %format, "Tracing initialized");
	}
	Ok(())
}

/// Holds the OTLP tracer provider so it can be flushed/shutdown cleanly.
#[cfg(feature = "otlp")]
static OTLP_PROVIDER: once_cell::sync::OnceCell<opentelemetry_sdk::trace::TracerProvider> =
	once_cell::sync::OnceCell::new();

/// Build an OTLP export layer pointing at `endpoint`.
#[cfg(feature = "otlp")]
fn build_otlp_layer(
	endpoint: &str,
) -> Result<
	Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>,
	Box<dyn std::error::Error>,
> {
	use opentelemetry::trace::TracerProvider as _;
	use opentelemetry_otlp::WithExportConfig;
	use tracing_subscriber::Layer;

	let exporter = opentelemetry_otlp::SpanExporter::builder()
		.with_tonic()
		.with_endpoint(endpoint)
		.build()?;

	let provider = opentelemetry_sdk::trace::TracerProvider::builder()
		.with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
		.build();

	let tracer = provider.tracer("neo3-sdk");
	let _ = OTLP_PROVIDER.set(provider);

	Ok(tracing_opentelemetry::layer().with_tracer(tracer).boxed())
}

/// Add event to current span.
pub fn add_event(name: &str, attributes: Vec<(&str, String)>) {
	let details = attributes
		.into_iter()
		.map(|(key, value)| format!("{key}={value}"))
		.collect::<Vec<_>>()
		.join(" ");
	tracing::info!(event = name, details = details);
}

/// Set span status.
pub fn set_status(success: bool, message: Option<&str>) {
	if success {
		tracing::info!(message = message.unwrap_or("ok"), "Span status");
	} else {
		tracing::warn!(message = message.unwrap_or("failed"), "Span status");
	}
}

/// Record an error in the current span.
pub fn record_error(error: &dyn std::error::Error) {
	tracing::error!(error = %error, "Span error");
}

/// Flush tracing resources owned by this module.
pub fn shutdown() {
	#[cfg(feature = "otlp")]
	{
		if let Some(provider) = OTLP_PROVIDER.get() {
			if let Err(e) = provider.shutdown() {
				eprintln!("OTLP tracer provider shutdown error: {e}");
			}
		}
	}
	tracing::debug!("Tracing shutdown requested");
}

// ---------------------------------------------------------------------------
// Correlation / request ID propagation
// ---------------------------------------------------------------------------

thread_local! {
	/// Correlation ID bound to the current execution context (thread/async task).
	static CURRENT_CORRELATION_ID: std::cell::RefCell<Option<String>> =
		const { std::cell::RefCell::new(None) };
}

/// Generate a new random correlation ID (32 lowercase hex characters).
#[must_use]
pub fn new_correlation_id() -> String {
	use rand::RngCore;
	let mut bytes = [0u8; 16];
	rand::rng().fill_bytes(&mut bytes);
	hex::encode(bytes)
}

/// Return the correlation ID bound to the current context, if any.
#[must_use]
pub fn current_correlation_id() -> Option<String> {
	CURRENT_CORRELATION_ID.with(|cell| cell.borrow().clone())
}

/// RAII guard that restores the previous correlation ID when dropped.
///
/// Returned by [`set_correlation_id`].
#[derive(Debug)]
pub struct CorrelationGuard {
	previous: Option<String>,
}

impl Drop for CorrelationGuard {
	fn drop(&mut self) {
		let previous = self.previous.take();
		CURRENT_CORRELATION_ID.with(|cell| *cell.borrow_mut() = previous);
	}
}

/// Bind `id` as the current correlation ID for the enclosing scope.
///
/// The previous value is restored automatically when the returned
/// [`CorrelationGuard`] is dropped.
pub fn set_correlation_id(id: impl Into<String>) -> CorrelationGuard {
	let previous = CURRENT_CORRELATION_ID.with(|cell| cell.borrow_mut().replace(id.into()));
	CorrelationGuard { previous }
}

/// Create an `info_span` in the `neo.correlation` context tagged with `id`.
///
/// Every span/event created while this span is entered inherits the correlation
/// ID through the span tree, so a whole request can be filtered by a single ID.
#[must_use]
pub fn correlation_span(id: &str) -> tracing::Span {
	tracing::info_span!("neo.correlation", correlation_id = %id)
}

/// Run `f` inside a correlation span and bind `id` as the current correlation ID.
///
/// This is the primary entry point for request-scoped tracing: wrap the body of
/// a request handler and every nested span/event will carry `id`.
pub fn with_correlation_id<F, R>(id: &str, f: F) -> R
where
	F: FnOnce() -> R,
{
	let span = correlation_span(id);
	let _enter = span.enter();
	let _guard = set_correlation_id(id);
	f()
}

/// Return the `(header_name, value)` pair used to propagate the current
/// correlation ID on an outbound RPC/HTTP request.
///
/// Returns `None` when no correlation ID is bound to the current context. Wire
/// this into RPC clients so a single logical request can be traced end-to-end
/// across service boundaries:
///
/// ```
/// use neo3::monitoring::tracing::{with_correlation_id, correlation_header};
///
/// with_correlation_id("req-42", || {
///     let (name, value) = correlation_header().expect("id is bound in scope");
///     assert_eq!(name, "x-correlation-id");
///     assert_eq!(value, "req-42");
/// });
/// ```
#[must_use]
pub fn correlation_header() -> Option<(&'static str, String)> {
	current_correlation_id().map(|id| (CORRELATION_ID_HEADER, id))
}

/// Adopt a correlation ID received from an inbound request header.
///
/// If `header_value` is `Some` and non-empty it is bound as the current
/// correlation ID; otherwise a fresh ID is generated. Returns the resolved ID
/// and a [`CorrelationGuard`] that restores the previous value when dropped, so
/// a server can continue an upstream trace or start a new one transparently.
#[must_use]
pub fn adopt_or_new_correlation_id(header_value: Option<&str>) -> (String, CorrelationGuard) {
	let id = header_value
		.map(str::trim)
		.filter(|v| !v.is_empty())
		.map(ToString::to_string)
		.unwrap_or_else(new_correlation_id);
	let guard = set_correlation_id(id.clone());
	(id, guard)
}

/// Execute a block in a transaction tracing context.
#[macro_export]
macro_rules! trace_transaction {
	($tx_type:expr, $network:expr, $body:block) => {{
		let span = tracing::info_span!("neo.transaction", tx_type = $tx_type, network = $network);
		let _guard = span.enter();
		$body
	}};
}

/// Execute a block in an RPC tracing context.
#[macro_export]
macro_rules! trace_rpc {
	($method:expr, $endpoint:expr, $body:block) => {{
		let span = tracing::info_span!("neo.rpc", method = $method, endpoint = $endpoint);
		let _guard = span.enter();
		$body
	}};
}

/// Execute a block in a contract tracing context.
#[macro_export]
macro_rules! trace_contract {
	($contract:expr, $operation:expr, $body:block) => {{
		let span =
			tracing::info_span!("neo.contract", contract = $contract, operation = $operation);
		let _guard = span.enter();
		$body
	}};
}

/// Execute a block in a correlation-ID tracing context.
#[macro_export]
macro_rules! trace_correlation {
	($id:expr, $body:block) => {{
		let span = $crate::monitoring::tracing::correlation_span($id);
		let _guard = span.enter();
		let _corr = $crate::monitoring::tracing::set_correlation_id($id);
		$body
	}};
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use std::sync::{Arc, Mutex};

	// ---- (a) JSON formatter produces valid, structured JSON output ----------

	#[derive(Clone, Default)]
	struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

	impl std::io::Write for SharedBuffer {
		fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
			self.0.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(buf);
			Ok(buf.len())
		}
		fn flush(&mut self) -> std::io::Result<()> {
			Ok(())
		}
	}

	impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedBuffer {
		type Writer = SharedBuffer;
		fn make_writer(&'a self) -> Self::Writer {
			self.clone()
		}
	}

	#[test]
	fn json_formatter_emits_valid_structured_json() {
		let buffer = SharedBuffer::default();
		let subscriber = tracing_subscriber::fmt()
			.json()
			.with_writer(buffer.clone())
			.finish();

		tracing::subscriber::with_default(subscriber, || {
			tracing::info!(answer = 42, "json structured event");
		});

		let raw = buffer.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
		let text = String::from_utf8(raw).expect("utf8 output");
		let trimmed = text.trim();
		assert!(!trimmed.is_empty(), "expected JSON output, got empty buffer");

		// Must parse as a single JSON object (this is the core assertion).
		let value: serde_json::Value =
			serde_json::from_str(trimmed).expect("formatter output must be valid JSON");

		assert!(value.is_object(), "JSON log line must be an object");
		assert_eq!(value["level"].as_str(), Some("INFO"));
		let fields = &value["fields"];
		assert_eq!(fields["message"].as_str(), Some("json structured event"));
		assert_eq!(fields["answer"].as_i64(), Some(42));
	}

	#[test]
	fn json_formatter_captures_correlation_id_field() {
		let buffer = SharedBuffer::default();
		let subscriber = tracing_subscriber::fmt()
			.json()
			.with_writer(buffer.clone())
			.finish();

		tracing::subscriber::with_default(subscriber, || {
			with_correlation_id("req-abc-123", || {
				tracing::info!("event inside correlation scope");
			});
		});

		let raw = buffer.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
		let text = String::from_utf8(raw).expect("utf8 output");
		// The event is emitted while the correlation span is entered, so the span
		// list (which carries correlation_id) is part of the JSON payload.
		assert!(
			text.contains("correlation_id") && text.contains("req-abc-123"),
			"expected correlation id in JSON output, got: {text}"
		);
	}

	// ---- (b) Config parsing for the new fields ------------------------------

	#[test]
	fn log_format_parses_case_insensitively() {
		assert_eq!(LogFormat::parse("json"), LogFormat::Json);
		assert_eq!(LogFormat::parse("JSON"), LogFormat::Json);
		assert_eq!(LogFormat::parse("  Json  "), LogFormat::Json);
		assert_eq!(LogFormat::parse("text"), LogFormat::Text);
		assert_eq!(LogFormat::parse("nonsense"), LogFormat::Text);
	}

	#[test]
	fn log_format_display_roundtrips() {
		assert_eq!(LogFormat::parse(&LogFormat::Json.to_string()), LogFormat::Json);
		assert_eq!(LogFormat::parse(&LogFormat::Text.to_string()), LogFormat::Text);
	}

	#[test]
	fn log_format_serde_roundtrips() {
		let json = serde_json::to_string(&LogFormat::Json).expect("serialize");
		assert_eq!(json, "\"json\"");
		let back: LogFormat = serde_json::from_str(&json).expect("deserialize");
		assert_eq!(back, LogFormat::Json);
	}

	#[test]
	fn config_builder_honors_log_format() {
		let cfg = crate::monitoring::MonitoringConfig::builder()
			.log_format(LogFormat::Json)
			.log_level("debug".to_string())
			.build();
		assert_eq!(cfg.log_format, LogFormat::Json);
		assert_eq!(cfg.log_level, "debug");

		// Default (unset) falls back to Text so existing behavior is preserved.
		let default_cfg = crate::monitoring::MonitoringConfig::builder().build();
		assert_eq!(default_cfg.log_format, LogFormat::Text);
	}

	// ---- (c) Correlation-ID propagation attaches the ID to spans ------------

	#[derive(Clone, Default)]
	struct SpanCapture(Arc<Mutex<Vec<(String, HashMap<String, String>)>>>);

	struct FieldVisitor(HashMap<String, String>);

	impl tracing::field::Visit for FieldVisitor {
		fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
			self.0.insert(field.name().to_string(), format!("{value:?}"));
		}
		fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
			self.0.insert(field.name().to_string(), value.to_string());
		}
	}

	impl<S> tracing_subscriber::Layer<S> for SpanCapture
	where
		S: tracing::Subscriber,
	{
		fn on_new_span(
			&self,
			attrs: &tracing::span::Attributes<'_>,
			_id: &tracing::span::Id,
			_ctx: tracing_subscriber::layer::Context<'_, S>,
		) {
			let mut visitor = FieldVisitor(HashMap::new());
			attrs.record(&mut visitor);
			self.0
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.push((attrs.metadata().name().to_string(), visitor.0));
		}
	}

	#[test]
	fn correlation_id_is_attached_to_span() {
		use tracing_subscriber::prelude::*;

		let capture = SpanCapture::default();
		let subscriber = tracing_subscriber::registry().with(capture.clone());

		tracing::subscriber::with_default(subscriber, || {
			with_correlation_id("req-xyz-789", || {
				// The correlation ID is also visible to application code.
				assert_eq!(current_correlation_id().as_deref(), Some("req-xyz-789"));
				tracing::info!("inside correlated scope");
			});
			// Restored once the scope exits.
			assert_eq!(current_correlation_id(), None);
		});

		let spans = capture.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
		let correlation = spans
			.iter()
			.find(|(name, _)| name == "neo.correlation")
			.expect("a neo.correlation span should have been created");
		assert_eq!(
			correlation.1.get(CORRELATION_ID_FIELD).map(String::as_str),
			Some("req-xyz-789"),
			"correlation_id field must be attached to the span"
		);
	}

	#[test]
	fn correlation_guard_restores_previous_id() {
		assert_eq!(current_correlation_id(), None);
		{
			let _outer = set_correlation_id("outer");
			assert_eq!(current_correlation_id().as_deref(), Some("outer"));
			{
				let _inner = set_correlation_id("inner");
				assert_eq!(current_correlation_id().as_deref(), Some("inner"));
			}
			assert_eq!(current_correlation_id().as_deref(), Some("outer"));
		}
		assert_eq!(current_correlation_id(), None);
	}

	#[test]
	fn new_correlation_id_is_unique_hex() {
		let a = new_correlation_id();
		let b = new_correlation_id();
		assert_eq!(a.len(), 32);
		assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
		assert_ne!(a, b, "generated ids should be unique");
	}

	// ---- (d) OTLP endpoint resolution ---------------------------------------

	#[test]
	fn resolve_otlp_endpoint_prefers_configured_value() {
		// A non-empty configured endpoint always wins, regardless of env.
		assert_eq!(
			resolve_otlp_endpoint("http://collector:4317"),
			Some("http://collector:4317".to_string())
		);
	}

	#[test]
	fn resolve_otlp_endpoint_trims_and_rejects_blank() {
		assert_eq!(
			resolve_otlp_endpoint("  http://c:4317  "),
			Some("http://c:4317".to_string())
		);
		// Blank configured + no env var (cleared) => None.
		std::env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
		assert_eq!(resolve_otlp_endpoint("   "), None);
	}

	// ---- (e) Correlation ID cross-process propagation helpers ---------------

	#[test]
	fn correlation_header_reflects_current_scope() {
		// Outside any scope there is no header to propagate.
		assert_eq!(current_correlation_id(), None);
		assert!(correlation_header().is_none());

		with_correlation_id("req-propagate-1", || {
			let (name, value) = correlation_header().expect("id bound in scope");
			assert_eq!(name, CORRELATION_ID_HEADER);
			assert_eq!(value, "req-propagate-1");
		});

		// Restored after the scope exits.
		assert!(correlation_header().is_none());
	}

	#[test]
	fn adopt_reuses_inbound_header_or_generates() {
		// Inbound header is adopted verbatim.
		{
			let (id, _guard) = adopt_or_new_correlation_id(Some("upstream-abc"));
			assert_eq!(id, "upstream-abc");
			assert_eq!(current_correlation_id().as_deref(), Some("upstream-abc"));
		}
		assert_eq!(current_correlation_id(), None);

		// Missing/blank inbound header => fresh generated id.
		{
			let (id, _guard) = adopt_or_new_correlation_id(None);
			assert_eq!(id.len(), 32);
			assert_eq!(current_correlation_id().as_deref(), Some(id.as_str()));
		}
		{
			let (id, _guard) = adopt_or_new_correlation_id(Some("   "));
			assert_eq!(id.len(), 32, "blank header should generate a new id");
		}
	}

	#[test]
	fn correlation_id_propagates_through_nested_rpc_spans() {
		use tracing_subscriber::prelude::*;

		let capture = SpanCapture::default();
		let subscriber = tracing_subscriber::registry().with(capture.clone());

		tracing::subscriber::with_default(subscriber, || {
			with_correlation_id("trace-across-rpc", || {
				// Simulate an outbound RPC call creating its own span while the
				// correlation scope is active; the header must carry the id so the
				// remote peer can continue the same logical trace.
				let rpc_span =
					tracing::info_span!("neo.rpc", method = "getblockcount", endpoint = "seed1");
				let _enter = rpc_span.enter();
				let (_, value) = correlation_header().expect("id available inside rpc span");
				assert_eq!(value, "trace-across-rpc");
				tracing::info!("rpc issued");
			});
		});

		let spans = capture.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
		// Both the correlation span and the nested rpc span were recorded.
		assert!(spans.iter().any(|(name, _)| name == "neo.correlation"));
		assert!(spans.iter().any(|(name, _)| name == "neo.rpc"));
	}
}
