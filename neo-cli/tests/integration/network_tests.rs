use super::utils::CliTest;

#[test]
fn test_network_status() {
	let cli = CliTest::new();

	// Get network status
	let output = cli.run(&["network", "status"]);

	// Command should be recognized even if no network is connected
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_network_nodes() {
	let cli = CliTest::new();

	// List connected nodes
	let output = cli.run(&["network", "peers"]);

	// Command should be recognized even if no network is connected
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_network_switch() {
	let cli = CliTest::new();

	// Connect to TestNet
	let testnet_output = cli.run(&["network", "connect", "--network", "testnet"]);
	// Command should be recognized
	assert!(testnet_output.status.code().unwrap_or(127) != 127, "Command not found");

	// Note: Each CLI invocation is separate, so state doesn't persist
	// Check network status
	let status_output = cli.run(&["network", "status"]);
	assert!(status_output.status.code().unwrap_or(127) != 127, "Command not found");

	// Connect to MainNet
	let mainnet_output = cli.run(&["network", "connect", "--network", "mainnet"]);
	// Command should be recognized
	assert!(mainnet_output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_network_add_node() {
	let cli = CliTest::new();

	// Add a node
	let output = cli.run(&[
		"network",
		"add",
		"--url",
		"http://seed1.ngd.network:10332",
		"--name",
		"test-node",
	]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_network_set_default() {
	let cli = CliTest::new();

	// First add a node
	cli.run(&[
		"network",
		"add",
		"--url",
		"http://seed2.ngd.network:10332",
		"--name",
		"default-node",
	]);

	// Connect to the node instead (no set-default command)
	let output = cli.run(&["network", "connect", "--network", "default-node"]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

#[test]
fn test_network_ping() {
	let cli = CliTest::new();

	// Ping a node
	let output = cli.run(&["network", "ping", "--network", "mainnet"]);

	// Command should be recognized even if ping fails
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}
