// Distributed tracing for NeoRust SDK
// Uses OpenTelemetry with Jaeger backend

use opentelemetry::{
    global,
    sdk::{
        export::trace::stdout,
        propagation::TraceContextPropagator,
        trace::{self, RandomIdGenerator, Sampler},
        Resource,
    },
    trace::{Span, SpanKind, Status, TraceContextExt, Tracer},
    KeyValue,
};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;

/// Initialize tracing system
pub fn init(endpoint: &str, log_level: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Set global trace context propagator
    global::set_text_map_propagator(TraceContextPropagator::new());
    
    // Create OTLP exporter
    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .with_protocol(Protocol::Grpc)
        .with_timeout(std::time::Duration::from_secs(3));
    
    // Create tracer
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(16)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "neorust"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    // Create telemetry layer
    let telemetry = OpenTelemetryLayer::new(tracer);
    
    // Create env filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));
    
    // Initialize subscriber
    Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

/// Create a new span for transaction processing
pub fn transaction_span(tx_type: &str, network: &str) -> impl Span {
    tracing::info_span!(
        "transaction",
        tx.type = %tx_type,
        neo.network = %network,
        otel.kind = ?SpanKind::Internal,
    )
}

/// Create a new span for RPC calls
pub fn rpc_span(method: &str, endpoint: &str) -> impl Span {
    tracing::info_span!(
        "rpc_call",
        rpc.method = %method,
        rpc.endpoint = %endpoint,
        otel.kind = ?SpanKind::Client,
    )
}

/// Create a new span for contract operations
pub fn contract_span(contract: &str, operation: &str) -> impl Span {
    tracing::info_span!(
        "contract_operation",
        contract.address = %contract,
        contract.operation = %operation,
        otel.kind = ?SpanKind::Internal,
    )
}

/// Create a new span for wallet operations
pub fn wallet_span(operation: &str, address: Option<&str>) -> impl Span {
    let span = tracing::info_span!(
        "wallet_operation",
        wallet.operation = %operation,
        otel.kind = ?SpanKind::Internal,
    );
    
    if let Some(addr) = address {
        span.record("wallet.address", &addr);
    }
    
    span
}

/// Add event to current span
pub fn add_event(name: &str, attributes: Vec<(&str, String)>) {
    tracing::event!(
        tracing::Level::INFO,
        name = %name,
        "{}", attributes.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Set span status
pub fn set_status(success: bool, message: Option<&str>) {
    if success {
        tracing::event!(tracing::Level::INFO, "Operation completed successfully");
    } else {
        tracing::event!(
            tracing::Level::ERROR,
            error.message = message.unwrap_or("Operation failed"),
            "Operation failed"
        );
    }
}

/// Record an error in the current span
pub fn record_error(error: &dyn std::error::Error) {
    tracing::error!(
        error.message = %error,
        error.source = ?error.source(),
        "Error occurred"
    );
}

/// Extract trace context from headers
pub fn extract_context(headers: &http::HeaderMap) -> opentelemetry::Context {
    let extractor = opentelemetry_http::HeaderExtractor(headers);
    global::get_text_map_propagator(|propagator| {
        propagator.extract(&extractor)
    })
}

/// Inject trace context into headers
pub fn inject_context(context: &opentelemetry::Context, headers: &mut http::HeaderMap) {
    let mut injector = opentelemetry_http::HeaderInjector(headers);
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(context, &mut injector);
    });
}

/// Shutdown tracing system
pub fn shutdown() {
    global::shutdown_tracer_provider();
}

// Helper macros for common tracing patterns
#[macro_export]
macro_rules! trace_transaction {
    ($tx_type:expr, $network:expr, $body:block) => {{
        let span = $crate::monitoring::tracing::transaction_span($tx_type, $network);
        let _guard = span.enter();
        $body
    }};
}

#[macro_export]
macro_rules! trace_rpc {
    ($method:expr, $endpoint:expr, $body:block) => {{
        let span = $crate::monitoring::tracing::rpc_span($method, $endpoint);
        let _guard = span.enter();
        $body
    }};
}

#[macro_export]
macro_rules! trace_contract {
    ($contract:expr, $operation:expr, $body:block) => {{
        let span = $crate::monitoring::tracing::contract_span($contract, $operation);
        let _guard = span.enter();
        $body
    }};
}