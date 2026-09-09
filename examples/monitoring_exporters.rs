//! # Production Monitoring Exporters Example
//!
//! This example demonstrates the SDK's production-ready monitoring export
//! capabilities added in v3.3.0. All features are feature-gated behind
//! `metrics-prometheus` and `metrics-otlp`.
//!
//! ```bash
//! cargo run --example monitoring_exporters --features "metrics-prometheus"
//! ```
//!
//! Note: this example requires tokio to be available (reused from existing deps).

#![allow(dead_code)] // entire module is just examples; compile only when features enabled

// =============================================================================
// SECTION 1: Basic Prometheus Metrics Exporter
// =============================================================================

/// Initialize the in-process metrics registry and start the `/metrics` scrape
/// endpoint on port 9090.
#[cfg(feature = "metrics-prometheus")]
async fn basic_prometheus() -> Result<(), Box<dyn std::error::Error>> {
    use neo3::monitoring::{metrics, prometheus};

    // Start the SDK's internal counters/gauges/histograms
    metrics::init(0)?;

    // Record some sample metrics matching common Neo usage patterns
    metrics::increment_counter("rpc.mainnet.getblock.success", 1.0);
    metrics::increment_counter("rpc.testnet.getblock.count", 5.0);
    metrics::set_gauge("blockchain.mainnet.height", 1234567.0);
    metrics::observe_histogram("transactions.mainnet.transfer.duration_seconds", 0.025);
    metrics::observe_histogram("transactions.mainnet.transfer.duration_seconds", 0.031);

    // Serve the /metrics endpoint
    prometheus::serve(prometheus::PrometheusConfig::from_env()).await?;

    // In real apps: run your HTTP server / keep task alive
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    metrics::shutdown();
    Ok(())
}

// =============================================================================
// SECTION 2: Filtering & Authentication
// =============================================================================

/// Use query parameters to filter scraped metrics and configure bearer token auth.
#[cfg(feature = "metrics-prometheus")]
async fn filtered_metrics_with_auth() -> Result<(), Box<dyn std::error::Error>> {
    use neo3::monitoring::prometheus;

    let config = prometheus::PrometheusConfig {
        port: 9090,
        auth_token: Some("my-secret-token".to_string()),
        namespace: "neorust".to_string(),
        enabled: true,
    };
    prometheus::serve(config).await?;
    Ok(())
}

// =============================================================================
// SECTION 3: Health Check Endpoints
// =============================================================================

/// Kubernetes-style liveness/readiness probes backed by the health registry.
#[cfg(feature = "metrics-prometheus")]
async fn health_probes() -> Result<(), Box<dyn std::error::Error>> {
    use neo3::monitoring::health_server;

    let config = health_server::HealthServerConfig {
        port: 8080,
        memory_degraded_pct: 70,
        memory_unhealthy_pct: 85,
    };
    health_server::serve(config).await?;

    // Track active RPC connections for load reporting
    health_server::connection_opened();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    health_server::connection_closed();

    // Keep running while health endpoints serve traffic
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    Ok(())
}

// =============================================================================
// SECTION 4: OTLP Trace Export with Correlation IDs
// =============================================================================

/// Propagate correlation IDs across request boundaries for end-to-end tracing.
#[cfg(all(feature = "metrics-prometheus"))] // correlation helpers don't require otlp, but we show both
async fn correlation_propagation() -> Result<(), Box<dyn std::error::Error>> {
    use neo3::monitoring::tracing::{
        with_correlation_id,
        correlation_header,
        adopt_or_new_correlation_id,
    };

    // Server side: continue upstream trace or generate fresh ID
    let (_id, _guard) = adopt_or_new_correlation_id(Some("upstream-trace-123"));

    // Client side: attach id to outbound RPC calls
    with_correlation_id("req-block-query", || {
        if let Some((name, value)) = correlation_header() {
            eprintln!("RPC header: {}={}", name, value);
        }
    });

    // Generate fresh IDs when none exists
    with_correlation_id("", || {
        let (_id, _guard) = adopt_or_new_correlation_id(None);
        // Fresh id generated
    });

    Ok(())
}

// =============================================================================
// SECTION 5: Integration Example
// =============================================================================

/// Full integration demo: metrics + health + correlation
#[cfg(feature = "metrics-prometheus")]
async fn full_integration_demo() -> Result<(), Box<dyn std::error::Error>> {
    use neo3::monitoring::{metrics, prometheus, health_server};

    // Initialize metrics registry
    metrics::init(0)?;

    // Simulate monitoring workload
    for i in 0..10 {
        metrics::increment_counter("integration.test.count", 1.0);
        metrics::set_gauge("integration.counter", i as f64);
        metrics::observe_histogram("integration.latency_ms", 10.0 * i as f64);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Start exporter endpoints
    prometheus::serve(prometheus::PrometheusConfig {
        port: 9090,
        ..Default::default()
    }).await?;

    health_server::serve(health_server::HealthServerConfig {
        port: 8080,
        ..Default::default()
    }).await?;

    // Keep runtime alive
    tokio::signal::ctrl_c().await.ok();
    Ok(())
}

// =============================================================================
// MAIN
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("NeoRust Monitoring Exporters Example");
    println!("=====================================");
    println!("\nThis example shows how to use production-grade exporters.");
    println!("All features are optional and feature-gated.\n");

    #[cfg(not(feature = "metrics-prometheus"))]
    {
        println!("⚠️  Feature 'metrics-prometheus' is required!");
        println!("\nTo enable exports, build with:");
        println!("  cargo run --example monitoring_exporters --features \"metrics-prometheus\"");
        return Ok(());
    }

    println!("✓ Feature 'metrics-prometheus' enabled");
    println!("  - /metrics endpoint on port 9090");
    println!("  - /healthz, /readyz, /status on port 8080");
    println!("\nTry these commands:");
    println!("  curl http://localhost:9090/metrics");
    println!("  curl http://localhost:8080/healthz");
    println!("  curl http://localhost:8080/readyz");
    println!("  curl http://localhost:8080/status");
    println!("\nPress Ctrl+C after verifying endpoints respond...\n");

    // Run demos sequentially
    match basic_prometheus().await {
        Ok(_) => println!("✓ Basic Prometheus example completed"),
        Err(e) => eprintln!("✗ Error in basic_prometheus: {}", e),
    }

    match health_probes().await {
        Ok(_) => println!("✓ Health probes example completed"),
        Err(e) => eprintln!("✗ Error in health_probes: {}", e),
    }

    // Don't actually block forever—just give time for manual testing
    tokio::time::timeout(tokio::time::Duration::from_secs(5), tokio::signal::ctrl_c())
        .await
        .ok();

    Ok(())
}
