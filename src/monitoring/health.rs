//! # Health Checks
//!
//! In-process health check infrastructure for liveness and readiness probes.
//! Applications can expose the returned registry state through their own HTTP
//! stack without coupling the SDK to a web framework.

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

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
	#[serde(skip)]
	pub last_check: Option<Instant>,
	pub metadata: HashMap<String, String>,
}

/// Health registry
pub struct HealthRegistry {
	checks: RwLock<HashMap<String, HealthCheck>>,
}

impl HealthRegistry {
	fn new() -> Self {
		Self { checks: RwLock::new(HashMap::new()) }
	}

	/// Register a health check
	pub fn register(&self, name: String, check: HealthCheck) {
		let mut checks = self.checks.write().unwrap_or_else(|e| e.into_inner());
		checks.insert(name, check);
	}

	/// Update health check status
	pub fn update(&self, name: &str, status: HealthStatus, message: Option<String>) {
		let mut checks = self.checks.write().unwrap_or_else(|e| e.into_inner());
		if let Some(check) = checks.get_mut(name) {
			check.status = status;
			check.message = message;
			check.last_check = Some(Instant::now());
		}
	}

	/// Get overall health status
	pub fn overall_status(&self) -> HealthStatus {
		let checks = self.checks.read().unwrap_or_else(|e| e.into_inner());

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
		let checks = self.checks.read().unwrap_or_else(|e| e.into_inner());
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

/// Initialize health check system.
pub fn init(port: u16) -> Result<(), Box<dyn std::error::Error>> {
	let registry = Arc::new(HealthRegistry::new());
	HEALTH_REGISTRY.set(registry).map_err(|_| "Health checks already initialized")?;

	register_default_checks(port);

	Ok(())
}

/// Register default health checks.
fn register_default_checks(port: u16) {
	if let Some(registry) = HEALTH_REGISTRY.get() {
		let mut process_metadata = HashMap::new();
		process_metadata.insert("configured_port".to_string(), port.to_string());
		registry.register(
			"sdk_process".to_string(),
			HealthCheck {
				name: "SDK Process".to_string(),
				status: HealthStatus::Healthy,
				message: Some("Health registry initialized".to_string()),
				last_check: Some(Instant::now()),
				metadata: process_metadata,
			},
		);

		registry.register(
			"memory".to_string(),
			HealthCheck {
				name: "Memory Usage".to_string(),
				status: HealthStatus::Degraded,
				message: Some("Memory check has not run yet".to_string()),
				last_check: Some(Instant::now()),
				metadata: HashMap::new(),
			},
		);
		check_memory_health(registry);
	}
}

/// Mark RPC connection health as degraded until an application reports a concrete probe result.
#[allow(dead_code)]
fn check_rpc_health(registry: &Arc<HealthRegistry>) {
	registry.update(
		"rpc_connection",
		HealthStatus::Degraded,
		Some("No RPC probe result has been registered".to_string()),
	);
}

/// Check memory usage health.
#[allow(dead_code)]
fn check_memory_health(registry: &Arc<HealthRegistry>) {
	let Some(memory_usage_percent) = current_memory_usage_percent() else {
		registry.update(
			"memory",
			HealthStatus::Degraded,
			Some("Memory usage is unavailable on this platform".to_string()),
		);
		return;
	};

	let (status, message) = if memory_usage_percent < 70 {
		(HealthStatus::Healthy, format!("Memory usage at {}%", memory_usage_percent))
	} else if memory_usage_percent < 85 {
		(HealthStatus::Degraded, format!("Memory usage elevated at {}%", memory_usage_percent))
	} else {
		(HealthStatus::Unhealthy, format!("Memory usage critical at {}%", memory_usage_percent))
	};

	registry.update("memory", status, Some(message));
}

#[cfg(target_os = "linux")]
fn current_memory_usage_percent() -> Option<u64> {
	let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
	let mut total_kb = None;
	let mut available_kb = None;
	for line in meminfo.lines() {
		let mut parts = line.split_whitespace();
		match parts.next()? {
			"MemTotal:" => total_kb = parts.next()?.parse::<u64>().ok(),
			"MemAvailable:" => available_kb = parts.next()?.parse::<u64>().ok(),
			_ => {},
		}
	}
	let total = total_kb?;
	let available = available_kb?;
	if total == 0 {
		return None;
	}
	Some(((total - available) * 100) / total)
}

#[cfg(not(target_os = "linux"))]
fn current_memory_usage_percent() -> Option<u64> {
	None
}

/// Mark blockchain sync health as degraded until an application reports a concrete probe result.
#[allow(dead_code)]
fn check_blockchain_health(registry: &Arc<HealthRegistry>) {
	registry.update(
		"blockchain_sync",
		HealthStatus::Degraded,
		Some("No blockchain sync probe result has been registered".to_string()),
	);
}

/// Update a health check
pub fn update_health(name: &str, status: HealthStatus, message: Option<String>) {
	if let Some(registry) = HEALTH_REGISTRY.get() {
		registry.update(name, status, message);
	}
}

/// Return the overall aggregated health status across every registered check.
///
/// Returns [`HealthStatus::Healthy`] when the registry has not been initialized
/// so that a not-yet-configured process still reports liveness.
#[must_use]
pub fn overall_status() -> HealthStatus {
	match HEALTH_REGISTRY.get() {
		Some(registry) => registry.overall_status(),
		None => HealthStatus::Healthy,
	}
}

/// Return a clone of every registered [`HealthCheck`].
#[must_use]
pub fn all_checks() -> Vec<HealthCheck> {
	match HEALTH_REGISTRY.get() {
		Some(registry) => registry.get_all(),
		None => Vec::new(),
	}
}

/// Report the current process memory usage as a percentage (0-100).
///
/// Returns `None` on platforms where this cannot be determined.
#[must_use]
pub fn memory_usage_percent() -> Option<u64> {
	current_memory_usage_percent()
}

/// Register a custom health check
pub fn register_health_check(name: String, initial_status: HealthStatus) {
	if let Some(registry) = HEALTH_REGISTRY.get() {
		registry.register(
			name.clone(),
			HealthCheck {
				name,
				status: initial_status,
				message: None,
				last_check: Some(Instant::now()),
				metadata: HashMap::new(),
			},
		);
	}
}

/// Shutdown health check system.
pub fn shutdown() {
	if let Some(registry) = HEALTH_REGISTRY.get() {
		registry.update("sdk_process", HealthStatus::Degraded, Some("Shutting down".to_string()));
	}
}
