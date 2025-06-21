#[cfg(test)]
mod tests {
	use crate::integration::utils::{assert_output_contains, assert_success, CliTest};
	use std::fs;

	#[test]
	fn test_init_default() {
		let cli = CliTest::new();

		// Run the init command with default parameters (use --force in case config exists)
		let output = cli.run_command(&["init", "--force"]);

		// Check that the command executed successfully
		assert_output_contains(&output, "Configuration initialized");
	}

	#[test]
	fn test_init_custom_path() {
		let cli = CliTest::new();

		// Construct a custom path for the configuration file
		let config_path = cli.temp_dir.path().join("custom");
		fs::create_dir_all(&config_path).expect("Failed to create directory");
		let config_file = config_path.join("config.json");

		// Run the init command with a custom path
		let output = cli.run_command(&["init", "--path", config_file.to_str().unwrap()]);

		// Check that the command executed successfully
		assert_success(&output);
		assert_output_contains(&output, "Configuration initialized");

		// Verify that the configuration file was created
		assert!(config_file.exists(), "Configuration file was not created");
	}
}
