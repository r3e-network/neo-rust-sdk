# SDK Monitoring Exporters (Prometheus / OTLP / Health)

This guide documents the production-ready export capabilities added to the
`neo3` crate's [`monitoring`](../../src/monitoring/) module and how they plug
into the observability stack in this directory (Prometheus, Grafana, Loki,
Jaeger/Tempo).

All exporters are **purely additive** and feature-gated — the default build is
unchanged.

## Feature flags

| Feature | Enables | Extra dependencies |
|---------|---------|--------------------|
| `metrics-prometheus` | `/metrics`, `/healthz`, `/readyz`, `/status` HTTP endpoints | none (reuses `tokio` + `serde_json`) |
| `metrics-otlp` | OTLP trace export (alias of the existing `otlp` feature) | `opentelemetry*`, `tracing-opentelemetry` |

```toml
[dependencies]
neo3 = { version = "3.3", features = ["metrics-prometheus", "metrics-otlp"] }
```

## 1. Prometheus metrics endpoint

The adapter renders the SDK's in-process counters/gauges/histograms in the
Prometheus text exposition format and serves them for scraping.

```rust,no_run
# #[cfg(feature = "metrics-prometheus")]
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
use neo3::monitoring::{metrics, prometheus};

metrics::init(0)?;                       // start the in-process registry
metrics::increment_counter("rpc.mainnet.getblockcount.success", 1.0);

// Serve GET /metrics on :9090 (spawns a background task, returns immediately).
prometheus::serve(prometheus::PrometheusConfig::from_env()).await?;
# Ok(())
# }
```

### Configuration (environment variables)

| Variable | Default | Meaning |
|----------|---------|---------|
| `NEO_PROMETHEUS_ENABLED` | `true` | Enable the adapter |
| `NEO_PROMETHEUS_PORT` | `9090` | HTTP listen port |
| `NEO_PROMETHEUS_AUTH_TOKEN` | _unset_ | Require `Authorization: Bearer <token>` |
| `NEO_METRICS_NAMESPACE` | _unset_ | Prefix prepended to every metric name |

### Filtering

Scrape a subset of series with query parameters:

- `GET /metrics?namespace=rpc` — only metrics whose name starts with `rpc`
- `GET /metrics?component=transfer` — only metrics whose name contains `transfer`

The existing [`prometheus/prometheus.yml`](../prometheus/prometheus.yml)
already scrapes `host.docker.internal:9090` under the `neorust-sdk` job, so the
default port works out of the box with the bundled stack.

### Authentication

When `NEO_PROMETHEUS_AUTH_TOKEN` is set, requests must send a matching
`Authorization: Bearer <token>` header. Configure Prometheus with:

```yaml
scrape_configs:
  - job_name: 'neorust-sdk'
    authorization:
      type: Bearer
      credentials: '<token>'
    static_configs:
      - targets: ['host.docker.internal:9090']
```

## 2. Health check endpoints

Kubernetes-style liveness/readiness probes backed by the SDK's health registry.

```rust,no_run
# #[cfg(feature = "metrics-prometheus")]
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
use neo3::monitoring::health_server::{self, HealthServerConfig};

health_server::serve(HealthServerConfig::from_env()).await?;
# Ok(())
# }
```

| Endpoint | Meaning | 200 when |
|----------|---------|----------|
| `GET /healthz` (`/livez`, `/health`) | Liveness | aggregated health ≠ `Unhealthy` |
| `GET /readyz` (`/ready`) | Readiness | all checks `Healthy` **and** memory below threshold |
| `GET /status` | Detailed JSON snapshot | always (diagnostics) |

The `/status` and `/readyz` payloads report service status, memory usage
percentage, active connection count, and per-check detail. Track connection
load from application code:

```rust,no_run
use neo3::monitoring::health_server;

health_server::connection_opened();
// ... use the connection ...
health_server::connection_closed();
```

### Thresholds (environment variables)

| Variable | Default | Meaning |
|----------|---------|---------|
| `NEO_HEALTH_CHECK_PORT` | `8080` | HTTP listen port |
| `NEO_HEALTH_MEMORY_DEGRADED_PCT` | `70` | ≥ this memory% ⇒ degraded |
| `NEO_HEALTH_MEMORY_UNHEALTHY_PCT` | `85` | ≥ this memory% ⇒ readiness fails |

## 3. OTLP trace export & correlation IDs

With `metrics-otlp` the tracing subscriber attaches an OpenTelemetry OTLP layer
that exports spans over gRPC. The endpoint is resolved in this order:

1. `NEO_TRACING_ENDPOINT`
2. `OTEL_EXPORTER_OTLP_ENDPOINT` (OpenTelemetry standard fallback)

```bash
export NEO_TRACING_ENABLED=true
export NEO_TRACING_ENDPOINT=http://otel-collector:4317
# or, using the OTel-standard variable:
export OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
```

### Correlation ID propagation across RPC calls

A single logical request can be traced end-to-end:

```rust
use neo3::monitoring::tracing::{
    with_correlation_id, correlation_header, adopt_or_new_correlation_id,
};

// Server side: continue an upstream trace or start a new one.
let (_id, _guard) = adopt_or_new_correlation_id(Some("upstream-req-42"));

// Client side: attach the current id to an outbound RPC request.
with_correlation_id("req-42", || {
    if let Some((name, value)) = correlation_header() {
        // e.g. request.header(name, value)
        assert_eq!(name, "x-correlation-id");
        assert_eq!(value, "req-42");
    }
});
```

The id also flows through the in-process span tree, so nested RPC/transaction
spans automatically carry it in logs (Loki) and traces (Jaeger/Tempo).

## 4. Log aggregation (Loki) & traces (Tempo)

- **Loki**: emit JSON logs (`NEO_LOG_FORMAT=json`) and ship them via the bundled
  [`promtail`](../promtail/promtail-config.yml). The OTEL collector also exports
  logs to Loki directly (see [`otel-collector-config.yml`](../otel/otel-collector-config.yml)).
- **Tempo**: an optional trace backend alongside Jaeger. Use
  [`tempo/tempo-config.yml`](../tempo/tempo-config.yml) and the pre-provisioned
  Grafana `Tempo` datasource. Correlation IDs recorded on spans let you jump
  from a Loki log line to the corresponding Tempo trace.

## End-to-end quick start

```bash
# 1. Start the observability stack
docker-compose -f monitoring/docker-compose.yml up -d

# 2. Run your app with exporters enabled
export NEO_LOG_FORMAT=json
export NEO_PROMETHEUS_PORT=9090
export NEO_HEALTH_CHECK_PORT=8080
export NEO_TRACING_ENDPOINT=http://localhost:4317
cargo run --features "metrics-prometheus,metrics-otlp" --bin your-app

# 3. Verify
curl -s localhost:9090/metrics | head
curl -s localhost:8080/healthz
curl -s localhost:8080/readyz
```
