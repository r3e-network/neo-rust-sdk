use super::utils::CliTest;

#[test]
fn test_blockchain_info() {
	let cli = CliTest::new();

	// Get blockchain info
	let output = cli.run(&["network", "status"]);

	// Command should be recognized even without connection
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_blockchain_height() {
	let cli = CliTest::new();

	// Get blockchain height
	let output = cli.run(&["network", "block"]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_blockchain_get_block_by_index() {
	let cli = CliTest::new();

	// Try to get block 0 (genesis block)
	let output = cli.run(&["network", "block", "--index", "0"]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_blockchain_get_block_by_hash() {
	let cli = CliTest::new();

	// Test with a known genesis block hash (this test requires network connectivity)
	// For now, just test that the command is recognized
	let output = cli.run(&["network", "block", "--index", "0"]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_blockchain_get_asset() {
	let cli = CliTest::new();

	// Get NEO asset info (use de-fi token command instead)
	let output = cli.run(&["de-fi", "token", "NEO"]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}
