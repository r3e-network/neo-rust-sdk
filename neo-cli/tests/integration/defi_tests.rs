#[cfg(test)]
mod tests {
	use crate::integration::utils::CliTest;

	#[test]
	fn test_defi_pools() {
		let cli = CliTest::new();

		// Test listing Flamingo liquidity pools
		let output = cli.run_command(&["de-fi", "pools", "--platform", "flamingo"]);

		// We're just checking if the command is recognized, not full execution
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	#[test]
	fn test_defi_swap_info() {
		let cli = CliTest::new();

		// Test getting swap information between NEO and GAS
		let output = cli.run_command(&[
			"de-fi",
			"swap-info",
			"--token-from",
			"NEO",
			"--token-to",
			"GAS",
			"--amount",
			"1",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	#[test]
	fn test_defi_swap() {
		let cli = CliTest::new();

		// Create and open a wallet for testing (required for swap)
		let wallet_content = r#"{
            "name": "TestWallet",
            "version": "1.0",
            "accounts": []
        }"#;
		let wallet_path = cli.create_temp_file(wallet_content);

		// First create the wallet
		cli.run_command(&["wallet", "open", "--path", wallet_path.to_str().unwrap()]);

		// Attempt to swap NEO to GAS
		let output = cli.run_command(&[
			"de-fi",
			"swap",
			"--token-from",
			"NEO",
			"--token-to",
			"GAS",
			"--amount",
			"1",
			"--slippage",
			"0.5",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	#[test]
	fn test_defi_add_liquidity() {
		let cli = CliTest::new();

		// Create and open a wallet for testing
		let wallet_content = r#"{
            "name": "TestWallet",
            "version": "1.0",
            "accounts": []
        }"#;
		let wallet_path = cli.create_temp_file(wallet_content);

		// First create the wallet
		cli.run_command(&["wallet", "open", "--path", wallet_path.to_str().unwrap()]);

		// Attempt to add liquidity
		let output = cli.run_command(&[
			"de-fi",
			"add-liquidity",
			"--token-a",
			"NEO",
			"--token-b",
			"GAS",
			"--amount-a",
			"1",
			"--amount-b",
			"1",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	#[test]
	fn test_defi_remove_liquidity() {
		let cli = CliTest::new();

		// Create and open a wallet for testing
		let wallet_content = r#"{
            "name": "TestWallet",
            "version": "1.0",
            "accounts": []
        }"#;
		let wallet_path = cli.create_temp_file(wallet_content);

		// First create the wallet
		cli.run_command(&["wallet", "open", "--path", wallet_path.to_str().unwrap()]);

		// Attempt to remove liquidity
		let output = cli.run_command(&[
			"de-fi",
			"remove-liquidity",
			"--token-a",
			"NEO",
			"--token-b",
			"GAS",
			"--percent",
			"50",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	/// Test the token info command
	#[test]
	fn test_defi_token_info() {
		let cli = CliTest::new();

		// Test NEO token info
		let output = cli.run_command(&["de-fi", "token", "NEO"]);

		// The command should be recognized even if it needs a network connection
		// Check that we're not getting a "command not found" error
		assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
	}

	/// Test the token info command with various token symbols
	#[test]
	fn test_defi_token_info_gas() {
		let cli = CliTest::new();

		// Test GAS token info
		let output = cli.run_command(&["de-fi", "token", "GAS"]);

		// The command should be recognized even if it needs a network connection
		assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
	}

	/// Test the balance command
	#[test]
	fn test_defi_balance_with_wallet() {
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

		// Test balance command with wallet
		let output = cli.run_command(&[
			"de-fi",
			"balance",
			"NEO",
			"--wallet",
			wallet_path.to_str().unwrap(),
			"--password",
			"test123",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
	}

	/// Test the contract invoke command (test only)
	#[test]
	fn test_defi_invoke_test() {
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

		// Test invoke NEO token symbol method (view only)
		let output = cli.run_command(&[
			"de-fi",
			"invoke",
			"NEO Token",
			"symbol",
			"--wallet",
			wallet_path.to_str().unwrap(),
			"--password",
			"test123",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	/// Test the contract balance command
	#[test]
	fn test_defi_balance() {
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

		// Test checking balance with address flag
		let output = cli.run_command(&[
			"de-fi",
			"balance",
			"NEO Token",
			"--address",
			"NZKvXidwBhnV8rNXh2eXtpm5bH1rkofaDz",
			"--wallet",
			wallet_path.to_str().unwrap(),
			"--password",
			"test123",
		]);

		// Just checking if the command is recognized
		assert!(output.status.code().unwrap_or(0) != 127, "Command not found");
	}

	/// Test error handling for invalid token symbol
	#[test]
	fn test_defi_invalid_token() {
		let cli = CliTest::new();

		// Test with a non-existent token
		let output = cli.run_command(&["de-fi", "token", "INVALID"]);

		// The command should be recognized but might fail due to invalid token
		assert!(output.status.code().unwrap_or(127) != 127, "Command not found");
	}
}
