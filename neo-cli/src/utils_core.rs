// Core utilities for CLI operations
use colored::*;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use std::{
	io::{self, Write},
	sync::atomic::{AtomicBool, Ordering},
	thread,
	time::Duration,
};

/// Print an informational message with icon
pub fn print_info(message: &str) {
	println!("{} {}", "ℹ".bright_blue(), message);
}

/// Print a success message with icon
pub fn print_success(message: &str) {
	println!("{} {}", "✅".bright_green(), message.bright_green());
}

/// Print an error message with icon
pub fn print_error(message: &str) {
	eprintln!("{} {}", "❌".bright_red(), message.bright_red());
}

/// Print a warning message with icon
pub fn print_warning(message: &str) {
	println!("{} {}", "⚠️".bright_yellow(), message.bright_yellow());
}

/// Print a debug message (only in verbose mode)
#[allow(dead_code)]
pub fn print_debug(message: &str) {
	if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
		println!("{} {}", "🔍".bright_magenta(), message.bright_black());
	}
}

/// Create a beautiful table for displaying data
pub fn create_table() -> Table {
	let mut table = Table::new();
	table
		.load_preset(UTF8_FULL)
		.set_content_arrangement(ContentArrangement::Dynamic)
		.set_header(vec![
			Cell::new("Field").add_attribute(Attribute::Bold).fg(Color::Cyan),
			Cell::new("Value").add_attribute(Attribute::Bold).fg(Color::Cyan),
		]);
	table
}

/// Create a progress bar with custom style
#[allow(dead_code)]
pub fn create_progress_bar(len: u64, message: &str) -> LightweightProgress {
	LightweightProgress::new(len, message)
}

/// Create an indeterminate spinner
pub fn create_spinner(message: &str) -> LightweightProgress {
	let mut pb = LightweightProgress::new(0, message);
	pb.enable_spinner();
	pb
}

/// Prompt user for yes/no confirmation with custom message
pub fn prompt_yes_no(message: &str) -> Result<bool, io::Error> {
	Confirm::with_theme(&ColorfulTheme::default())
		.with_prompt(message)
		.default(false)
		.interact()
		.map_err(|e| io::Error::other(e))
}

/// Prompt user for password input
pub fn prompt_password(message: &str) -> Result<String, io::Error> {
	Password::with_theme(&ColorfulTheme::default())
		.with_prompt(message)
		.interact()
		.map_err(|e| io::Error::other(e))
}

/// Prompt user for text input
pub fn prompt_input(message: &str) -> Result<String, io::Error> {
	Input::with_theme(&ColorfulTheme::default())
		.with_prompt(message)
		.interact_text()
		.map_err(|e| io::Error::other(e))
}

/// Prompt user for text input with default value
#[allow(dead_code)]
pub fn prompt_input_with_default(message: &str, default: &str) -> Result<String, io::Error> {
	Input::with_theme(&ColorfulTheme::default())
		.with_prompt(message)
		.default(default.to_string())
		.interact_text()
		.map_err(|e| io::Error::other(e))
}

/// Prompt user to select from a list of options
pub fn prompt_select(message: &str, options: &[&str]) -> Result<usize, io::Error> {
	Select::with_theme(&ColorfulTheme::default())
		.with_prompt(message)
		.items(options)
		.default(0)
		.interact()
		.map_err(|e| io::Error::other(e))
}

/// Minimal progress indicator to avoid heavy dependencies.
pub struct LightweightProgress {
	len: u64,
	pos: u64,
	message: String,
	spinner: bool,
	stop_flag: Option<std::sync::Arc<AtomicBool>>,
	worker: Option<thread::JoinHandle<()>>,
}

impl LightweightProgress {
	pub fn new(len: u64, message: &str) -> Self {
		Self {
			len,
			pos: 0,
			message: message.to_string(),
			spinner: false,
			stop_flag: None,
			worker: None,
		}
	}

	pub fn enable_spinner(&mut self) {
		if self.spinner {
			return;
		}
		self.spinner = true;
		let message = self.message.clone();
		let stop = std::sync::Arc::new(AtomicBool::new(false));
		let stop_handle = stop.clone();
		self.stop_flag = Some(stop);
		self.worker = Some(thread::spawn(move || {
			let frames = ['⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈'];
			let mut idx = 0;
			while !stop_handle.load(Ordering::Relaxed) {
				print!("\r{} {}", frames[idx], message);
				let _ = io::stdout().flush();
				idx = (idx + 1) % frames.len();
				thread::sleep(Duration::from_millis(120));
			}
		}));
	}

	#[allow(dead_code)]
	pub fn inc(&mut self, delta: u64) {
		self.pos = self.pos.saturating_add(delta).min(self.len);
		self.render();
	}

	pub fn finish_with_message(&mut self, msg: &str) {
		self.message = msg.to_string();
		self.pos = self.len;
		self.stop_spinner();
		if self.spinner {
			print!("\r✔ {}", self.message);
		} else {
			self.render();
		}
		println!();
	}

	fn stop_spinner(&mut self) {
		if let Some(flag) = self.stop_flag.take() {
			flag.store(true, Ordering::Relaxed);
		}
		if let Some(handle) = self.worker.take() {
			let _ = handle.join();
		}
	}

	fn render(&mut self) {
		if self.len > 0 {
			print!("\r[{}/{}] {}", self.pos, self.len, self.message);
		} else {
			print!("\r{}", self.message);
		}
		let _ = io::stdout().flush();
	}
}

impl Drop for LightweightProgress {
	fn drop(&mut self) {
		self.stop_spinner();
	}
}

/// Display a formatted key-value pair
pub fn display_key_value(key: &str, value: &str) {
	println!("{}: {}", key.bright_cyan().bold(), value.bright_white());
}

/// Display a formatted key-value pair with custom colors
#[allow(dead_code)]
pub fn display_key_value_colored(key: &str, value: &str, key_color: Color, value_color: Color) {
	let key_colored = match key_color {
		Color::Red => key.bright_red(),
		Color::Green => key.bright_green(),
		Color::Blue => key.bright_blue(),
		Color::Yellow => key.bright_yellow(),
		Color::Magenta => key.bright_magenta(),
		Color::Cyan => key.bright_cyan(),
		_ => key.bright_white(),
	};

	let value_colored = match value_color {
		Color::Red => value.bright_red(),
		Color::Green => value.bright_green(),
		Color::Blue => value.bright_blue(),
		Color::Yellow => value.bright_yellow(),
		Color::Magenta => value.bright_magenta(),
		Color::Cyan => value.bright_cyan(),
		_ => value.bright_white(),
	};

	println!("{}: {}", key_colored.bold(), value_colored);
}

/// Format a large number with thousands separators
#[allow(dead_code)]
pub fn format_number(num: u64) -> String {
	num.to_string()
		.as_bytes()
		.rchunks(3)
		.rev()
		.map(std::str::from_utf8)
		.collect::<Result<Vec<&str>, _>>()
		.unwrap()
		.join(",")
}

/// Format bytes in human-readable format
#[allow(dead_code)]
pub fn format_bytes(bytes: u64) -> String {
	const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
	let mut size = bytes as f64;
	let mut unit_index = 0;

	while size >= 1024.0 && unit_index < UNITS.len() - 1 {
		size /= 1024.0;
		unit_index += 1;
	}

	if unit_index == 0 {
		format!("{} {}", size as u64, UNITS[unit_index])
	} else {
		format!("{:.2} {}", size, UNITS[unit_index])
	}
}

/// Format duration in human-readable format
#[allow(dead_code)]
pub fn format_duration(seconds: u64) -> String {
	let days = seconds / 86400;
	let hours = (seconds % 86400) / 3600;
	let minutes = (seconds % 3600) / 60;
	let secs = seconds % 60;

	if days > 0 {
		format!("{}d {}h {}m {}s", days, hours, minutes, secs)
	} else if hours > 0 {
		format!("{}h {}m {}s", hours, minutes, secs)
	} else if minutes > 0 {
		format!("{}m {}s", minutes, secs)
	} else {
		format!("{}s", secs)
	}
}

/// Print a section header
pub fn print_section_header(title: &str) {
	println!();
	println!("{}", title.bright_green().bold().underline());
	println!("{}", "─".repeat(title.len()).bright_green());
}

/// Print a subsection header
#[allow(dead_code)]
pub fn print_subsection_header(title: &str) {
	println!();
	println!("{}", title.bright_cyan().bold());
}

/// Clear the terminal screen
#[allow(dead_code)]
pub fn clear_screen() {
	print!("\x1B[2J\x1B[1;1H");
	io::stdout().flush().unwrap();
}

/// Wait for user to press Enter
#[allow(dead_code)]
pub fn wait_for_enter(message: Option<&str>) {
	let msg = message.unwrap_or("Press Enter to continue...");
	print!("{} ", msg.bright_yellow());
	io::stdout().flush().unwrap();
	let mut input = String::new();
	io::stdin().read_line(&mut input).unwrap();
}

/// Ensure account is loaded with proper validation
pub fn ensure_account_loaded() -> Result<(), crate::errors::CliError> {
	// This will be implemented when we have proper account management
	Ok(())
}

/// Display a loading animation for async operations
pub async fn with_loading<F, T>(message: &str, future: F) -> T
where
	F: std::future::Future<Output = T>,
{
	let mut spinner = create_spinner(message);
	let result = future.await;
	spinner.finish_with_message(&format!("{} ✅", message));
	result
}

/// Display error details in a formatted way
#[allow(dead_code)]
pub fn display_error_details(error: &dyn std::error::Error) {
	print_error(&format!("Error: {}", error));

	let mut source = error.source();
	let mut level = 1;

	while let Some(err) = source {
		println!(
			"{}{} Caused by: {}",
			"  ".repeat(level),
			"└─".bright_red(),
			err.to_string().bright_red()
		);
		source = err.source();
		level += 1;
	}
}

/// Create a status indicator
pub fn status_indicator(status: &str) -> ColoredString {
	match status.to_lowercase().as_str() {
		"success" | "ok" | "active" | "online" => "●".bright_green(),
		"warning" | "pending" | "syncing" => "●".bright_yellow(),
		"error" | "failed" | "offline" => "●".bright_red(),
		"info" | "unknown" => "●".bright_blue(),
		_ => "●".bright_white(),
	}
}
