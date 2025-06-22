use std::{
	io::Write,
	path::PathBuf,
	process::{Command, Output},
};
use tempfile::{NamedTempFile, TempDir};

pub struct CliTest {
	/// Temporary directory for test files
	pub temp_dir: TempDir,
	/// Path to the cli binary
	pub binary_path: PathBuf,
}

impl CliTest {
	/// Create a new test environment
	pub fn new() -> Self {
		let bin_path = std::env::current_dir().unwrap();
		let temp_dir = TempDir::new().expect("Failed to create temp directory");

		Self { temp_dir, binary_path: bin_path }
	}

	/// Run a CLI command with the given arguments
	pub fn run(&self, args: &[&str]) -> Output {
		let mut cmd = Command::new("cargo");
		cmd.arg("run")
			.arg("--")
			.args(args)
			.current_dir(&self.binary_path);
		
		// Set HOME environment variable for Windows compatibility
		if cfg!(target_os = "windows") {
			if let Ok(userprofile) = std::env::var("USERPROFILE") {
				cmd.env("HOME", userprofile);
			}
		}
		
		cmd.output().expect("Failed to execute command")
	}

	/// Alias for run to match what's used in tests
	pub fn run_command(&self, args: &[&str]) -> Output {
		self.run(args)
	}

	/// Create a temporary file with the given content
	pub fn create_temp_file(&self, content: &str) -> PathBuf {
		let mut file = NamedTempFile::new_in(&self.temp_dir).unwrap();
		file.write_all(content.as_bytes()).unwrap();
		let path = file.into_temp_path();
		let path_buf = path.to_path_buf();
		path.keep().unwrap();
		path_buf
	}
}

/// Helper function to assert that command was successful
pub fn assert_success(output: &Output) {
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(output.status.success(), "Command failed: {stderr}");
}

/// Helper function to assert command output contains a string
pub fn assert_output_contains(output: &Output, expected: &str) {
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(
		stdout.contains(expected),
		"Expected output to contain '{expected}', but got:\n{stdout}"
	);
}
