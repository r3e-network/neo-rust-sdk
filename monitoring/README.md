# NeoRust Monitoring System

A comprehensive monitoring infrastructure for the NeoRust SDK and its components.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Application Layer                        │
├───────────────┬─────────────────┬───────────────────────────┤
│   Neo SDK     │    Neo CLI      │       Neo GUI            │
│   Metrics     │    Metrics      │       Metrics            │
└───────┬───────┴────────┬────────┴────────┬──────────────────┘
        │                │                 │
┌───────▼────────────────▼─────────────────▼──────────────────┐
│              Metrics Collection Layer (OpenTelemetry)        │
├───────────────────────────────────────────────────────────────┤
│   • Traces  • Metrics  • Logs  • Distributed Context        │
└───────────────────────────┬───────────────────────────────────┘
                           │
┌───────────────────────────▼───────────────────────────────────┐
│                    Storage & Analysis                         │
├─────────────┬──────────────┬─────────────┬──────────────────┤
│  Prometheus │   Grafana    │   Jaeger    │   Loki           │
│  (Metrics)  │ (Dashboards) │  (Traces)   │  (Logs)          │
└─────────────┴──────────────┴─────────────┴──────────────────┘
```

## Components

### 1. Metrics Collection
- **OpenTelemetry**: Unified observability framework
- **Custom Exporters**: Neo blockchain specific metrics
- **Performance Counters**: Transaction processing, RPC calls
- **Resource Metrics**: Memory, CPU, network usage

### 2. Dashboards
- **System Health**: Overall system status
- **Performance**: Response times, throughput
- **Blockchain**: Block height, sync status, transaction metrics
- **Error Rates**: Failed transactions, connection errors
- **Business Metrics**: Active wallets, token transfers

### 3. Alerting Rules
- Critical: System down, blockchain disconnected
- Warning: High error rates, performance degradation
- Info: Configuration changes, version updates

### 4. Log Aggregation
- Structured logging with correlation IDs
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Automatic error context capture
- Performance profiling logs

## Quick Start

### Local Development
```bash
# Start monitoring stack
docker-compose -f monitoring/docker-compose.yml up -d

# View dashboards
open http://localhost:3000  # Grafana
open http://localhost:9090  # Prometheus
open http://localhost:16686 # Jaeger
```

### Production Deployment
See [deployment guide](./docs/deployment.md) for production setup.

## Integration

### Rust SDK
```rust
use neo3::monitoring::metrics;

// Initialize monitoring
metrics::init();

// Record custom metric
metrics::counter!("neo_transactions_total", 1);
metrics::histogram!("transaction_duration_seconds", duration);
```

### CLI
```bash
# Enable monitoring
neo-cli --metrics-port 9091 wallet create

# Export metrics endpoint
export NEO_CLI_METRICS_ENABLED=true
```

### GUI
The GUI automatically exports metrics on port 9092 when running.

## Metrics Reference

See [metrics documentation](./docs/metrics.md) for complete metric definitions.

## Alerts Reference

See [alerts documentation](./docs/alerts.md) for alerting rules and thresholds.