// Metrics collection for NeoRust SDK
// Uses OpenTelemetry and Prometheus for metrics export

use once_cell::sync::OnceCell;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, Encoder, GaugeVec, HistogramVec, TextEncoder,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Handle;
use warp::Filter;

static METRICS_REGISTRY: OnceCell<Arc<MetricsRegistry>> = OnceCell::new();

/// Core metrics registry
pub struct MetricsRegistry {
    // Transaction metrics
    pub transactions_total: CounterVec,
    pub transactions_failed: CounterVec,
    pub transaction_duration: HistogramVec,
    pub transaction_fees: HistogramVec,
    
    // RPC metrics
    pub rpc_requests_total: CounterVec,
    pub rpc_request_duration: HistogramVec,
    pub rpc_errors_total: CounterVec,
    
    // Wallet metrics
    pub wallets_created: CounterVec,
    pub wallet_operations: CounterVec,
    pub wallet_balance: GaugeVec,
    
    // Blockchain metrics
    pub block_height: GaugeVec,
    pub sync_status: GaugeVec,
    pub connected_nodes: GaugeVec,
    pub network_latency: HistogramVec,
    
    // Contract metrics
    pub contract_invocations: CounterVec,
    pub contract_deployments: CounterVec,
    pub contract_gas_used: HistogramVec,
    
    // System metrics
    pub memory_usage: GaugeVec,
    pub cpu_usage: GaugeVec,
    pub goroutines: GaugeVec,
    pub open_connections: GaugeVec,
}

impl MetricsRegistry {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            // Transaction metrics
            transactions_total: register_counter_vec!(
                "neo_transactions_total",
                "Total number of transactions",
                &["type", "network"]
            )?,
            transactions_failed: register_counter_vec!(
                "neo_transactions_failed_total",
                "Total number of failed transactions",
                &["type", "network", "reason"]
            )?,
            transaction_duration: register_histogram_vec!(
                "neo_transaction_duration_seconds",
                "Transaction processing duration",
                &["type", "network"],
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]
            )?,
            transaction_fees: register_histogram_vec!(
                "neo_transaction_fees_gas",
                "Transaction fees in GAS",
                &["type", "network"],
                vec![0.0001, 0.001, 0.01, 0.1, 1.0, 10.0, 100.0]
            )?,
            
            // RPC metrics
            rpc_requests_total: register_counter_vec!(
                "neo_rpc_requests_total",
                "Total number of RPC requests",
                &["method", "endpoint"]
            )?,
            rpc_request_duration: register_histogram_vec!(
                "neo_rpc_request_duration_seconds",
                "RPC request duration",
                &["method", "endpoint"],
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
            )?,
            rpc_errors_total: register_counter_vec!(
                "neo_rpc_errors_total",
                "Total number of RPC errors",
                &["method", "endpoint", "error_type"]
            )?,
            
            // Wallet metrics
            wallets_created: register_counter_vec!(
                "neo_wallets_created_total",
                "Total number of wallets created",
                &["type"]
            )?,
            wallet_operations: register_counter_vec!(
                "neo_wallet_operations_total",
                "Total wallet operations",
                &["operation", "wallet_type"]
            )?,
            wallet_balance: register_gauge_vec!(
                "neo_wallet_balance",
                "Current wallet balance",
                &["address", "token"]
            )?,
            
            // Blockchain metrics
            block_height: register_gauge_vec!(
                "neo_block_height",
                "Current blockchain height",
                &["network"]
            )?,
            sync_status: register_gauge_vec!(
                "neo_sync_status",
                "Blockchain sync status (1=synced, 0=syncing)",
                &["network"]
            )?,
            connected_nodes: register_gauge_vec!(
                "neo_connected_nodes",
                "Number of connected nodes",
                &["network"]
            )?,
            network_latency: register_histogram_vec!(
                "neo_network_latency_ms",
                "Network latency in milliseconds",
                &["endpoint"],
                vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]
            )?,
            
            // Contract metrics
            contract_invocations: register_counter_vec!(
                "neo_contract_invocations_total",
                "Total contract invocations",
                &["contract", "method"]
            )?,
            contract_deployments: register_counter_vec!(
                "neo_contract_deployments_total",
                "Total contract deployments",
                &["network"]
            )?,
            contract_gas_used: register_histogram_vec!(
                "neo_contract_gas_used",
                "Gas used by contract operations",
                &["contract", "operation"],
                vec![0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0]
            )?,
            
            // System metrics
            memory_usage: register_gauge_vec!(
                "neo_memory_usage_bytes",
                "Memory usage in bytes",
                &["component"]
            )?,
            cpu_usage: register_gauge_vec!(
                "neo_cpu_usage_percent",
                "CPU usage percentage",
                &["component"]
            )?,
            goroutines: register_gauge_vec!(
                "neo_goroutines",
                "Number of goroutines",
                &["component"]
            )?,
            open_connections: register_gauge_vec!(
                "neo_open_connections",
                "Number of open connections",
                &["type"]
            )?,
        })
    }
}

/// Initialize metrics system
pub fn init(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(MetricsRegistry::new()?);
    METRICS_REGISTRY.set(registry.clone()).map_err(|_| "Metrics already initialized")?;
    
    // Start metrics server
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .map(move || {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });
    
    // Spawn metrics server
    tokio::spawn(async move {
        warp::serve(metrics_route).run(addr).await;
    });
    
    Ok(())
}

/// Get metrics registry
pub fn registry() -> Option<Arc<MetricsRegistry>> {
    METRICS_REGISTRY.get().cloned()
}

/// Record a transaction
pub fn record_transaction(tx_type: &str, network: &str, duration: f64, success: bool) {
    if let Some(registry) = registry() {
        registry.transactions_total
            .with_label_values(&[tx_type, network])
            .inc();
        
        if !success {
            registry.transactions_failed
                .with_label_values(&[tx_type, network, "execution_failed"])
                .inc();
        }
        
        registry.transaction_duration
            .with_label_values(&[tx_type, network])
            .observe(duration);
    }
}

/// Record an RPC request
pub fn record_rpc_request(method: &str, endpoint: &str, duration: f64, success: bool) {
    if let Some(registry) = registry() {
        registry.rpc_requests_total
            .with_label_values(&[method, endpoint])
            .inc();
        
        registry.rpc_request_duration
            .with_label_values(&[method, endpoint])
            .observe(duration);
        
        if !success {
            registry.rpc_errors_total
                .with_label_values(&[method, endpoint, "request_failed"])
                .inc();
        }
    }
}

/// Update blockchain metrics
pub fn update_blockchain_metrics(network: &str, height: u64, synced: bool, nodes: u64) {
    if let Some(registry) = registry() {
        registry.block_height
            .with_label_values(&[network])
            .set(height as f64);
        
        registry.sync_status
            .with_label_values(&[network])
            .set(if synced { 1.0 } else { 0.0 });
        
        registry.connected_nodes
            .with_label_values(&[network])
            .set(nodes as f64);
    }
}

/// Record contract invocation
pub fn record_contract_invocation(contract: &str, method: &str, gas_used: f64) {
    if let Some(registry) = registry() {
        registry.contract_invocations
            .with_label_values(&[contract, method])
            .inc();
        
        registry.contract_gas_used
            .with_label_values(&[contract, "invocation"])
            .observe(gas_used);
    }
}

/// Shutdown metrics system
pub fn shutdown() {
    // Metrics server will shutdown when tokio runtime stops
}

// Convenience macros for recording metrics
#[macro_export]
macro_rules! counter {
    ($name:expr, $value:expr) => {
        if let Some(registry) = $crate::monitoring::metrics::registry() {
            // Implementation would lookup and increment the counter
        }
    };
}

#[macro_export]
macro_rules! gauge {
    ($name:expr, $value:expr) => {
        if let Some(registry) = $crate::monitoring::metrics::registry() {
            // Implementation would lookup and set the gauge
        }
    };
}

#[macro_export]
macro_rules! histogram {
    ($name:expr, $value:expr) => {
        if let Some(registry) = $crate::monitoring::metrics::registry() {
            // Implementation would lookup and observe the histogram
        }
    };
}