// Monitoring module for NeoRust SDK
// Provides metrics, tracing, and observability features

pub mod metrics;
pub mod tracing;
pub mod health;

use once_cell::sync::Lazy;
use std::sync::Arc;

/// Global monitoring configuration
pub static MONITORING: Lazy<Arc<MonitoringConfig>> = Lazy::new(|| {
    Arc::new(MonitoringConfig::from_env())
});

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub metrics_enabled: bool,
    pub metrics_port: u16,
    pub tracing_enabled: bool,
    pub tracing_endpoint: String,
    pub log_level: String,
    pub health_check_enabled: bool,
    pub health_check_port: u16,
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
            log_level: std::env::var("NEO_LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
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

/// Initialize monitoring subsystems
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize metrics
    if MONITORING.metrics_enabled {
        metrics::init(MONITORING.metrics_port)?;
    }
    
    // Initialize tracing
    if MONITORING.tracing_enabled {
        tracing::init(&MONITORING.tracing_endpoint, &MONITORING.log_level)?;
    }
    
    // Initialize health checks
    if MONITORING.health_check_enabled {
        health::init(MONITORING.health_check_port)?;
    }
    
    Ok(())
}

/// Shutdown monitoring subsystems
pub fn shutdown() {
    metrics::shutdown();
    tracing::shutdown();
    health::shutdown();
}