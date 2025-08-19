# NeoRust Monitoring Integration Guide

## Quick Start

### 1. Add Dependencies

Add monitoring dependencies to your `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

# Monitoring
prometheus = "0.13"
opentelemetry = "0.20"
opentelemetry-otlp = "0.13"
opentelemetry-http = "0.9"
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-opentelemetry = "0.21"
warp = "0.3"  # For metrics endpoint
```

### 2. Initialize Monitoring

In your main application:

```rust
use neo3::monitoring;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize monitoring
    monitoring::init()?;
    
    // Your application code
    run_application().await?;
    
    // Shutdown monitoring gracefully
    monitoring::shutdown();
    Ok(())
}
```

### 3. Environment Variables

Configure monitoring through environment variables:

```bash
# Metrics
export NEO_METRICS_ENABLED=true
export NEO_METRICS_PORT=9090

# Tracing
export NEO_TRACING_ENABLED=true
export NEO_TRACING_ENDPOINT=http://localhost:4317

# Logging
export NEO_LOG_LEVEL=info

# Health Checks
export NEO_HEALTH_CHECK_ENABLED=true
export NEO_HEALTH_CHECK_PORT=8080
```

## Recording Metrics

### Transaction Metrics

```rust
use neo3::monitoring::metrics;
use std::time::Instant;

// Start timing
let start = Instant::now();

// Process transaction
let result = process_transaction(&tx).await;

// Record metrics
let duration = start.elapsed().as_secs_f64();
metrics::record_transaction(
    "transfer",  // transaction type
    "mainnet",   // network
    duration,    // duration in seconds
    result.is_ok()  // success
);
```

### RPC Metrics

```rust
use neo3::monitoring::metrics;

let start = Instant::now();
let response = rpc_client.call("getblock", params).await;

metrics::record_rpc_request(
    "getblock",
    "https://seed1.neo.org:10332",
    start.elapsed().as_secs_f64(),
    response.is_ok()
);
```

### Custom Metrics

```rust
use neo3::monitoring::metrics;

// Counter
counter!("custom_operations_total", 1);

// Gauge
gauge!("active_connections", 42);

// Histogram
histogram!("processing_time_seconds", 0.123);
```

## Distributed Tracing

### Creating Spans

```rust
use neo3::monitoring::tracing;
use tracing::instrument;

#[instrument]
async fn transfer_tokens(from: &str, to: &str, amount: u64) -> Result<()> {
    // Automatically traced
    validate_addresses(from, to)?;
    
    // Manual span
    let span = tracing::transaction_span("transfer", "mainnet");
    let _guard = span.enter();
    
    // Add events
    tracing::add_event("transfer_started", vec![
        ("from", from.to_string()),
        ("to", to.to_string()),
        ("amount", amount.to_string()),
    ]);
    
    // Process transfer
    let result = execute_transfer(from, to, amount).await;
    
    // Set status
    tracing::set_status(result.is_ok(), result.err().map(|e| e.to_string()).as_deref());
    
    result
}
```

### Trace Context Propagation

```rust
use neo3::monitoring::tracing;

// Extract context from incoming request
let context = tracing::extract_context(&request.headers());

// Use context for downstream calls
let mut headers = HeaderMap::new();
tracing::inject_context(&context, &mut headers);
```

## Health Checks

### Register Custom Health Checks

```rust
use neo3::monitoring::health;

// Register a custom check
health::register_health_check(
    "database".to_string(),
    health::HealthStatus::Healthy
);

// Update health status
health::update_health(
    "database",
    health::HealthStatus::Degraded,
    Some("High latency detected".to_string())
);
```

### Health Endpoints

The following endpoints are automatically available:

- `GET /health` - Overall health status with all checks
- `GET /health/liveness` - Kubernetes liveness probe
- `GET /health/readiness` - Kubernetes readiness probe

## Structured Logging

```rust
use tracing::{info, warn, error, debug};

// Log with structured fields
info!(
    transaction_id = %tx_id,
    amount = amount,
    network = "mainnet",
    "Transaction processed successfully"
);

// Log errors with context
error!(
    error = %e,
    transaction_id = %tx_id,
    "Transaction failed"
);
```

## Docker Deployment

### Start Monitoring Stack

```bash
# Start all monitoring services
cd monitoring
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f
```

### Access Dashboards

- Grafana: http://localhost:3000 (admin/neorust)
- Prometheus: http://localhost:9090
- Jaeger: http://localhost:16686
- AlertManager: http://localhost:9093

## Kubernetes Integration

### ConfigMap for Monitoring

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: neorust-monitoring
data:
  NEO_METRICS_ENABLED: "true"
  NEO_METRICS_PORT: "9090"
  NEO_TRACING_ENABLED: "true"
  NEO_TRACING_ENDPOINT: "http://otel-collector:4317"
  NEO_LOG_LEVEL: "info"
```

### Service Monitor for Prometheus

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: neorust
spec:
  selector:
    matchLabels:
      app: neorust
  endpoints:
  - port: metrics
    interval: 30s
    path: /metrics
```

## Performance Considerations

### Sampling

Configure trace sampling for production:

```rust
use opentelemetry::sdk::trace::{Sampler, Config};

let trace_config = Config::default()
    .with_sampler(Sampler::TraceIdRatioBased(0.1)); // Sample 10%
```

### Batching

Metrics and traces are automatically batched. Configure batch size:

```rust
let batch_config = BatchConfig::default()
    .with_max_export_batch_size(512)
    .with_max_queue_size(2048);
```

### Resource Limits

Set memory limits for monitoring components:

```yaml
resources:
  limits:
    memory: "512Mi"
    cpu: "500m"
  requests:
    memory: "256Mi"
    cpu: "250m"
```

## Troubleshooting

### Debug Monitoring

Enable debug logging:

```bash
export RUST_LOG=neo3::monitoring=debug
```

### Check Metrics Endpoint

```bash
curl http://localhost:9090/metrics
```

### Verify Trace Export

```bash
export OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://localhost:4317
export OTEL_TRACES_EXPORTER=console  # Print to console for debugging
```

### Common Issues

1. **Metrics not appearing**: Check firewall rules and port availability
2. **Traces not exported**: Verify OTLP endpoint connectivity
3. **High memory usage**: Adjust batch sizes and sampling rates
4. **Missing logs**: Ensure log level is set appropriately

## Best Practices

1. **Use structured logging** with consistent field names
2. **Add trace context** to all external API calls
3. **Set meaningful span names** that describe the operation
4. **Record business metrics** alongside technical metrics
5. **Use appropriate log levels** (ERROR for failures, WARN for degradation)
6. **Implement circuit breakers** for external dependencies
7. **Set up alerts** for critical business operations
8. **Regular review** of metrics and dashboard usage