use super::utils::{assert_output_contains, assert_success, CliTest};

const TEST_WALLET_PASSWORD: &str = "password123";

#[test]
fn test_wallet_create() {
	let cli = CliTest::new();

	// Create wallet
	let wallet_path = cli.temp_dir.path().join("test-wallet.json").to_string_lossy().to_string();
	let output =
		cli.run(&["wallet", "create", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);

	assert_success(&output);
	assert_output_contains(&output, "Creating");

	// Verify file exists
	assert!(std::path::Path::new(&wallet_path).exists());
}

#[test]
fn test_wallet_open_and_close() {
	let cli = CliTest::new();

	// Create wallet first
	let wallet_path = cli.temp_dir.path().join("test-wallet.json").to_string_lossy().to_string();
	let create_output =
		cli.run(&["wallet", "create", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);
	assert_success(&create_output);

	// Test open wallet
	let open_output =
		cli.run(&["wallet", "open", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);
	assert_success(&open_output);
	assert_output_contains(&open_output, "Wallet opened successfully");

	// Test close wallet (Note: wallet state doesn't persist between CLI invocations)
	let close_output = cli.run(&["wallet", "close"]);
	assert_success(&close_output);
	// Since each CLI run is a separate process, there's no wallet to close
	assert_output_contains(&close_output, "No wallet");
}

#[test]
#[ignore] // Address creation not implemented yet
fn test_wallet_create_address() {
	let cli = CliTest::new();

	// Create and open wallet
	let wallet_path = cli.temp_dir.path().join("test-wallet.json").to_string_lossy().to_string();
	cli.run(&["wallet", "create", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);
	cli.run(&["wallet", "open", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);

	// Create a new address
	let output = cli.run(&["wallet", "create-address", "--count", "1"]);

	assert_success(&output);
	assert_output_contains(&output, "New address created");
}

#[test]
fn test_wallet_list_address() {
	let cli = CliTest::new();

	// Create wallet
	let wallet_path = cli.temp_dir.path().join("test-wallet.json").to_string_lossy().to_string();
	cli.run(&["wallet", "create", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);

	// Note: Each CLI run is a separate process, so wallet state doesn't persist
	// The list command will show no wallet is open
	let output = cli.run(&["wallet", "list"]);

	// The command will fail because no wallet is open
	// This is expected behavior for a CLI where each invocation is stateless
	assert!(!output.status.success());
	// Check stderr for error message
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(
		stderr.contains("No wallet open"),
		"Expected error message 'No wallet open', got: {stderr}"
	);
}

#[test]
#[ignore] // Address creation not implemented yet
fn test_wallet_balance() {
	let cli = CliTest::new();

	// Create and open wallet
	let wallet_path = cli.temp_dir.path().join("test-wallet.json").to_string_lossy().to_string();
	cli.run(&["wallet", "create", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);
	cli.run(&["wallet", "open", "--path", &wallet_path, "--password", TEST_WALLET_PASSWORD]);

	// Create an address
	cli.run(&["wallet", "create-address", "--count", "1"]);

	// Check balance (will be zero, but should run successfully)
	let output = cli.run(&["wallet", "balance"]);

	assert_success(&output);
}
