#![allow(dead_code, unused_imports)]

use chrono::Local;
use env_logger::Builder;
use log::{Level, LevelFilter, Record};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Log format configuration
#[derive(Debug, Clone, Deserialize)]
pub enum LogFormat {
	Pretty,
	Json,
	Compact,
}

/// Logger configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LoggerConfig {
	pub level: String,
	pub format: LogFormat,
	pub file_path: Option<PathBuf>,
	pub enable_colors: bool,
	pub enable_timestamps: bool,
	pub enable_module_path: bool,
}

impl Default for LoggerConfig {
	fn default() -> Self {
		Self {
			level: "info".to_string(),
			format: LogFormat::Pretty,
			file_path: None,
			enable_colors: true,
			enable_timestamps: true,
			enable_module_path: false,
		}
	}
}

/// Structured log entry for JSON format
#[derive(Debug, Serialize)]
struct JsonLogEntry {
	timestamp: String,
	level: String,
	target: String,
	message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	module: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	file: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	line: Option<u32>,
}

/// Initialize the logger with configuration
pub fn init_logger(config: LoggerConfig) -> Result<(), Box<dyn std::error::Error>> {
	let level_filter = match config.level.to_lowercase().as_str() {
		"trace" => LevelFilter::Trace,
		"debug" => LevelFilter::Debug,
		"info" => LevelFilter::Info,
		"warn" | "warning" => LevelFilter::Warn,
		"error" => LevelFilter::Error,
		_ => LevelFilter::Info,
	};

	let file_writer = if let Some(path) = config.file_path.clone() {
		// Create parent directories if they don't exist
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}

		Some(Arc::new(Mutex::new(OpenOptions::new().create(true).append(true).open(path)?)))
	} else {
		None
	};

	let mut builder = Builder::new();
	builder.filter_level(level_filter);

	match config.format {
		LogFormat::Pretty => {
			builder.format(move |buf, record| {
				let mut style = String::new();
				let mut reset = String::new();

				if config.enable_colors {
					style = match record.level() {
						Level::Error => "\x1b[31m".to_string(), // Red
						Level::Warn => "\x1b[33m".to_string(),  // Yellow
						Level::Info => "\x1b[32m".to_string(),  // Green
						Level::Debug => "\x1b[36m".to_string(), // Cyan
						Level::Trace => "\x1b[90m".to_string(), // Gray
					};
					reset = "\x1b[0m".to_string();
				}

				let timestamp = if config.enable_timestamps {
					format!("[{}] ", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
				} else {
					String::new()
				};

				let module = if config.enable_module_path {
					format!(" [{}]", record.target())
				} else {
					String::new()
				};

				let log_line = format!(
					"{}{}{}{:5}{}{} {}\n",
					timestamp,
					style,
					record.level(),
					reset,
					module,
					if module.is_empty() { "" } else { " -" },
					record.args()
				);

				// Write to file if configured
				if let Some(ref file) = file_writer {
					if let Ok(mut f) = file.lock() {
						let _ = f.write_all(log_line.as_bytes());
					}
				}

				write!(buf, "{}", log_line)
			});
		},
		LogFormat::Json => {
			builder.format(move |buf, record| {
				let entry = JsonLogEntry {
					timestamp: Local::now().to_rfc3339(),
					level: record.level().to_string(),
					target: record.target().to_string(),
					message: format!("{}", record.args()),
					module: record.module_path().map(|s| s.to_string()),
					file: record.file().map(|s| s.to_string()),
					line: record.line(),
				};

				let json = serde_json::to_string(&entry)
					.unwrap_or_else(|_| format!("{{\"error\":\"Failed to serialize log entry\"}}"));

				// Write to file if configured
				if let Some(ref file) = file_writer {
					if let Ok(mut f) = file.lock() {
						let _ = writeln!(f, "{}", json);
					}
				}

				writeln!(buf, "{}", json)
			});
		},
		LogFormat::Compact => {
			builder.format(move |buf, record| {
				let log_line = format!(
					"{} {} {}\n",
					Local::now().format("%H:%M:%S"),
					record.level().to_string().chars().next().unwrap(),
					record.args()
				);

				// Write to file if configured
				if let Some(ref file) = file_writer {
					if let Ok(mut f) = file.lock() {
						let _ = f.write_all(log_line.as_bytes());
					}
				}

				write!(buf, "{}", log_line)
			});
		},
	}

	builder.init();
	Ok(())
}

/// Structured logger for specific contexts
pub struct StructuredLogger {
	context: std::collections::HashMap<String, String>,
}

impl StructuredLogger {
	pub fn new() -> Self {
		Self { context: std::collections::HashMap::new() }
	}

	pub fn with_context(mut self, key: &str, value: &str) -> Self {
		self.context.insert(key.to_string(), value.to_string());
		self
	}

	pub fn log(&self, level: Level, message: &str) {
		let context_str = if !self.context.is_empty() {
			let ctx: Vec<String> =
				self.context.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
			format!(" [{}]", ctx.join(", "))
		} else {
			String::new()
		};

		log::log!(level, "{}{}", message, context_str);
	}

	pub fn error(&self, message: &str) {
		self.log(Level::Error, message);
	}

	pub fn warn(&self, message: &str) {
		self.log(Level::Warn, message);
	}

	pub fn info(&self, message: &str) {
		self.log(Level::Info, message);
	}

	pub fn debug(&self, message: &str) {
		self.log(Level::Debug, message);
	}

	pub fn trace(&self, message: &str) {
		self.log(Level::Trace, message);
	}
}

/// Performance logger for tracking operation durations
pub struct PerformanceLogger {
	operation: String,
	start_time: std::time::Instant,
	threshold: std::time::Duration,
}

impl PerformanceLogger {
	pub fn new(operation: &str) -> Self {
		Self {
			operation: operation.to_string(),
			start_time: std::time::Instant::now(),
			threshold: std::time::Duration::from_secs(1),
		}
	}

	pub fn with_threshold(mut self, threshold: std::time::Duration) -> Self {
		self.threshold = threshold;
		self
	}

	pub fn complete(self) {
		let duration = self.start_time.elapsed();

		if duration > self.threshold {
			log::warn!(
				"Operation '{}' took {:?} (threshold: {:?})",
				self.operation,
				duration,
				self.threshold
			);
		} else {
			log::debug!("Operation '{}' completed in {:?}", self.operation, duration);
		}
	}

	pub fn complete_with_result<T>(self, result: &Result<T, impl std::error::Error>) {
		let duration = self.start_time.elapsed();

		match result {
			Ok(_) => {
				if duration > self.threshold {
					log::warn!(
						"Operation '{}' succeeded but took {:?} (threshold: {:?})",
						self.operation,
						duration,
						self.threshold
					);
				} else {
					log::debug!("Operation '{}' succeeded in {:?}", self.operation, duration);
				}
			},
			Err(e) => {
				log::error!("Operation '{}' failed after {:?}: {}", self.operation, duration, e);
			},
		}
	}
}

/// Audit logger for security-sensitive operations
pub struct AuditLogger {
	file_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct AuditEntry {
	timestamp: String,
	user: Option<String>,
	operation: String,
	resource: Option<String>,
	result: String,
	metadata: std::collections::HashMap<String, String>,
}

impl AuditLogger {
	pub fn new(file_path: PathBuf) -> Result<Self, std::io::Error> {
		// Create parent directories if they don't exist
		if let Some(parent) = file_path.parent() {
			std::fs::create_dir_all(parent)?;
		}

		Ok(Self { file_path })
	}

	pub fn log_operation(
		&self,
		user: Option<&str>,
		operation: &str,
		resource: Option<&str>,
		success: bool,
		metadata: std::collections::HashMap<String, String>,
	) -> Result<(), std::io::Error> {
		let entry = AuditEntry {
			timestamp: Local::now().to_rfc3339(),
			user: user.map(|s| s.to_string()),
			operation: operation.to_string(),
			resource: resource.map(|s| s.to_string()),
			result: if success { "success" } else { "failure" }.to_string(),
			metadata,
		};

		let json = serde_json::to_string(&entry)
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

		let mut file = OpenOptions::new().create(true).append(true).open(&self.file_path)?;

		writeln!(file, "{}", json)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_logger_initialization() {
		let config = LoggerConfig::default();
		let result = init_logger(config);
		assert!(result.is_ok());
	}

	#[test]
	fn test_structured_logger() {
		let logger = StructuredLogger::new()
			.with_context("user", "alice")
			.with_context("operation", "transfer");

		logger.info("Test message");
	}

	#[test]
	fn test_performance_logger() {
		let perf = PerformanceLogger::new("test_operation")
			.with_threshold(std::time::Duration::from_millis(100));

		std::thread::sleep(std::time::Duration::from_millis(50));
		perf.complete();
	}
}
