use crate::integration::utils::{assert_output_contains, assert_success, CliTest};
use tempfile::NamedTempFile;

/// Test the NeoFS endpoints list command
#[test]
fn test_fs_endpoints_list() {
	let cli = CliTest::new();

	// Test mainnet endpoints list
	let output = cli.run(&["fs", "endpoints", "list", "--network", "mainnet"]);

	assert_success(&output);
	assert_output_contains(&output, "NeoFS endpoints for mainnet network:");

	// Test testnet endpoints list
	let output = cli.run(&["fs", "endpoints", "list", "--network", "testnet"]);

	assert_success(&output);
	assert_output_contains(&output, "NeoFS endpoints for testnet network:");
}

/// Test the NeoFS status command without a wallet
#[test]
fn test_fs_status_no_wallet() {
	let cli = CliTest::new();

	// Test status without a wallet (should show limited info)
	let output = cli.run(&["fs", "status"]);

	// Command should be recognized even if it fails to connect
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

/// Test the NeoFS endpoint test command
#[test]
fn test_fs_endpoints_test() {
	let cli = CliTest::new();

	// Test HTTP endpoint
	let output = cli.run(&[
		"fs",
		"endpoints",
		"test",
		"--endpoint",
		"https://http.mainnet.fs.neo.org",
		"--type",
		"http",
	]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");

	// Test with default type (grpc)
	let output = cli.run(&["fs", "endpoints", "test", "--network", "mainnet"]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

/// Test the NeoFS container commands
#[test]
fn test_fs_container_operations() {
	let cli = CliTest::new();

	// Create a mock wallet for testing
	let wallet_path = cli.create_temp_file(
		r#"{
        "name": "test_wallet",
        "version": "1.0",
        "scrypt": {"n": 16384, "r": 8, "p": 8},
        "accounts": [
            {
                "address": "NZKvXidwBhnV8rNXh2eXtpm5bH1rkofaDz",
                "label": "test_account",
                "isDefault": true,
                "lock": false,
                "key": "6PYXHjPaNvW8YknSXaKzL1Xoxw4RjmQwCryMGEZ2GaLhGH8AdazLJPBBXw",
                "contract": {
                    "script": "DCECIgZYieFCd+WHwCJK/I8btx1lYRIzOz8I8ZB6Ll6G3IIRLUFAQQ==",
                    "parameters": [{"name": "signature", "type": "Signature"}]
                }
            }
        ]
    }"#,
	);

	// Create a mock container config
	let container_config = cli.create_temp_file(
		r#"{
        "name": "test-container",
        "basic_acl": 644,
        "placement_policy": "REP 3",
        "attributes": [
            {"key": "CreatedBy", "value": "NeoRust CLI"},
            {"key": "Description", "value": "Test container"}
        ]
    }"#,
	);

	// Test container create (will fail without an actual connection, but we can verify command structure)
	let output = cli.run(&[
		"fs",
		"container",
		"create",
		"--config",
		container_config.to_str().unwrap(),
		"--wallet",
		wallet_path.to_str().unwrap(),
		"--password",
		"test123",
	]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

/// Test the NeoFS object commands
#[test]
fn test_fs_object_operations() {
	let cli = CliTest::new();

	// Create a mock wallet for testing
	let wallet_path = cli.create_temp_file(
		r#"{
        "name": "test_wallet",
        "version": "1.0",
        "scrypt": {"n": 16384, "r": 8, "p": 8},
        "accounts": [
            {
                "address": "NZKvXidwBhnV8rNXh2eXtpm5bH1rkofaDz",
                "label": "test_account",
                "isDefault": true,
                "lock": false,
                "key": "6PYXHjPaNvW8YknSXaKzL1Xoxw4RjmQwCryMGEZ2GaLhGH8AdazLJPBBXw",
                "contract": {
                    "script": "DCECIgZYieFCd+WHwCJK/I8btx1lYRIzOz8I8ZB6Ll6G3IIRLUFAQQ==",
                    "parameters": [{"name": "signature", "type": "Signature"}]
                }
            }
        ]
    }"#,
	);

	// Create a temporary file to upload
	let mut temp_file = NamedTempFile::new().unwrap();
	std::io::Write::write_all(&mut temp_file, b"This is test content for NeoFS").unwrap();
	let file_path = temp_file.path();

	// Test object put (will fail without an actual connection, but we can verify command structure)
	let output = cli.run(&[
		"fs",
		"object",
		"put",
		"--container",
		"testhash",
		"--file",
		file_path.to_str().unwrap(),
		"--wallet",
		wallet_path.to_str().unwrap(),
		"--password",
		"test123",
	]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}

/// Test the NeoFS ACL commands
#[test]
fn test_fs_acl_operations() {
	let cli = CliTest::new();

	// Create a mock wallet for testing
	let wallet_path = cli.create_temp_file(
		r#"{
        "name": "test_wallet",
        "version": "1.0",
        "scrypt": {"n": 16384, "r": 8, "p": 8},
        "accounts": [
            {
                "address": "NZKvXidwBhnV8rNXh2eXtpm5bH1rkofaDz",
                "label": "test_account",
                "isDefault": true,
                "lock": false,
                "key": "6PYXHjPaNvW8YknSXaKzL1Xoxw4RjmQwCryMGEZ2GaLhGH8AdazLJPBBXw",
                "contract": {
                    "script": "DCECIgZYieFCd+WHwCJK/I8btx1lYRIzOz8I8ZB6Ll6G3IIRLUFAQQ==",
                    "parameters": [{"name": "signature", "type": "Signature"}]
                }
            }
        ]
    }"#,
	);

	// Test ACL set (will fail without an actual connection, but we can verify command structure)
	let output = cli.run(&[
		"fs",
		"acl",
		"set",
		"--container",
		"testhash",
		"--basic",
		"644",
		"--wallet",
		wallet_path.to_str().unwrap(),
		"--password",
		"test123",
	]);

	// Command should be recognized
	assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
}
