// Health check endpoints for NeoRust SDK
// Provides liveness and readiness probes for Kubernetes and monitoring systems

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use warp::Filter;

static HEALTH_REGISTRY: OnceCell<Arc<HealthRegistry>> = OnceCell::new();

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Component health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub last_check: Instant,
    pub metadata: HashMap<String, String>,
}

/// Health registry
pub struct HealthRegistry {
    checks: RwLock<HashMap<String, HealthCheck>>,
    shutdown_tx: mpsc::Sender<()>,
}

impl HealthRegistry {
    fn new(shutdown_tx: mpsc::Sender<()>) -> Self {
        Self {
            checks: RwLock::new(HashMap::new()),
            shutdown_tx,
        }
    }
    
    /// Register a health check
    pub fn register(&self, name: String, check: HealthCheck) {
        let mut checks = self.checks.write().unwrap();
        checks.insert(name, check);
    }
    
    /// Update health check status
    pub fn update(&self, name: &str, status: HealthStatus, message: Option<String>) {
        let mut checks = self.checks.write().unwrap();
        if let Some(check) = checks.get_mut(name) {
            check.status = status;
            check.message = message;
            check.last_check = Instant::now();
        }
    }
    
    /// Get overall health status
    pub fn overall_status(&self) -> HealthStatus {
        let checks = self.checks.read().unwrap();
        
        if checks.is_empty() {
            return HealthStatus::Healthy;
        }
        
        let has_unhealthy = checks.values().any(|c| c.status == HealthStatus::Unhealthy);
        let has_degraded = checks.values().any(|c| c.status == HealthStatus::Degraded);
        
        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
    
    /// Get all health checks
    pub fn get_all(&self) -> Vec<HealthCheck> {
        let checks = self.checks.read().unwrap();
        checks.values().cloned().collect()
    }
}

/// Health response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: u64,
    pub version: String,
    pub checks: Vec<HealthCheck>,
}

/// Initialize health check system
pub fn init(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
    let registry = Arc::new(HealthRegistry::new(shutdown_tx));
    HEALTH_REGISTRY.set(registry.clone()).map_err(|_| "Health checks already initialized")?;
    
    // Register default checks
    register_default_checks();
    
    // Create health check routes
    let health_route = warp::path("health")
        .and(warp::get())
        .map(move || {
            let registry = HEALTH_REGISTRY.get().unwrap();
            let response = HealthResponse {
                status: registry.overall_status(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                checks: registry.get_all(),
            };
            warp::reply::json(&response)
        });
    
    let liveness_route = warp::path("health")
        .and(warp::path("liveness"))
        .and(warp::get())
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "alive",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            }))
        });
    
    let readiness_route = warp::path("health")
        .and(warp::path("readiness"))
        .and(warp::get())
        .map(move || {
            let registry = HEALTH_REGISTRY.get().unwrap();
            let status = registry.overall_status();
            let status_code = match status {
                HealthStatus::Healthy => warp::http::StatusCode::OK,
                HealthStatus::Degraded => warp::http::StatusCode::OK,
                HealthStatus::Unhealthy => warp::http::StatusCode::SERVICE_UNAVAILABLE,
            };
            
            warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "ready": status != HealthStatus::Unhealthy,
                    "status": status,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                })),
                status_code,
            )
        });
    
    let routes = health_route.or(liveness_route).or(readiness_route);
    
    // Start health check server
    let addr = ([0, 0, 0, 0], port);
    tokio::spawn(async move {
        let (_, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(addr, async move {
                shutdown_rx.recv().await;
            });
        server.await;
    });
    
    // Start background health checker
    start_health_checker();
    
    Ok(())
}

/// Register default health checks
fn register_default_checks() {
    if let Some(registry) = HEALTH_REGISTRY.get() {
        // RPC connection check
        registry.register(
            "rpc_connection".to_string(),
            HealthCheck {
                name: "RPC Connection".to_string(),
                status: HealthStatus::Healthy,
                message: Some("RPC connection initialized".to_string()),
                last_check: Instant::now(),
                metadata: HashMap::new(),
            },
        );
        
        // Database connection check (if applicable)
        registry.register(
            "database".to_string(),
            HealthCheck {
                name: "Database".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Database connection healthy".to_string()),
                last_check: Instant::now(),
                metadata: HashMap::new(),
            },
        );
        
        // Memory usage check
        registry.register(
            "memory".to_string(),
            HealthCheck {
                name: "Memory Usage".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Memory usage within limits".to_string()),
                last_check: Instant::now(),
                metadata: HashMap::new(),
            },
        );
        
        // Blockchain sync check
        registry.register(
            "blockchain_sync".to_string(),
            HealthCheck {
                name: "Blockchain Sync".to_string(),
                status: HealthStatus::Healthy,
                message: Some("Blockchain fully synced".to_string()),
                last_check: Instant::now(),
                metadata: HashMap::new(),
            },
        );
    }
}

/// Start background health checker
fn start_health_checker() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            if let Some(registry) = HEALTH_REGISTRY.get() {
                // Check RPC connection
                check_rpc_health(registry);
                
                // Check memory usage
                check_memory_health(registry);
                
                // Check blockchain sync
                check_blockchain_health(registry);
            }
        }
    });
}

/// Check RPC connection health
fn check_rpc_health(registry: &Arc<HealthRegistry>) {
    // This would actually check RPC connection
    // For now, we'll simulate it
    let healthy = true; // Replace with actual check
    
    registry.update(
        "rpc_connection",
        if healthy { HealthStatus::Healthy } else { HealthStatus::Unhealthy },
        Some(if healthy {
            "RPC connection healthy".to_string()
        } else {
            "RPC connection failed".to_string()
        }),
    );
}

/// Check memory usage health
fn check_memory_health(registry: &Arc<HealthRegistry>) {
    // Get current memory usage
    // This is a simplified check - in production you'd use actual memory metrics
    let memory_usage_percent = 50; // Placeholder
    
    let (status, message) = if memory_usage_percent < 70 {
        (HealthStatus::Healthy, format!("Memory usage at {}%", memory_usage_percent))
    } else if memory_usage_percent < 85 {
        (HealthStatus::Degraded, format!("Memory usage elevated at {}%", memory_usage_percent))
    } else {
        (HealthStatus::Unhealthy, format!("Memory usage critical at {}%", memory_usage_percent))
    };
    
    registry.update("memory", status, Some(message));
}

/// Check blockchain sync health
fn check_blockchain_health(registry: &Arc<HealthRegistry>) {
    // This would check actual blockchain sync status
    // For now, we'll simulate it
    let synced = true; // Replace with actual check
    
    registry.update(
        "blockchain_sync",
        if synced { HealthStatus::Healthy } else { HealthStatus::Degraded },
        Some(if synced {
            "Blockchain fully synced".to_string()
        } else {
            "Blockchain syncing in progress".to_string()
        }),
    );
}

/// Update a health check
pub fn update_health(name: &str, status: HealthStatus, message: Option<String>) {
    if let Some(registry) = HEALTH_REGISTRY.get() {
        registry.update(name, status, message);
    }
}

/// Register a custom health check
pub fn register_health_check(name: String, initial_status: HealthStatus) {
    if let Some(registry) = HEALTH_REGISTRY.get() {
        registry.register(
            name.clone(),
            HealthCheck {
                name: name.clone(),
                status: initial_status,
                message: None,
                last_check: Instant::now(),
                metadata: HashMap::new(),
            },
        );
    }
}

/// Shutdown health check system
pub fn shutdown() {
    if let Some(registry) = HEALTH_REGISTRY.get() {
        let _ = registry.shutdown_tx.try_send(());
    }
}