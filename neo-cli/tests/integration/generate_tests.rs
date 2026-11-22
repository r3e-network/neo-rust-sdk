#[cfg(test)]
mod tests {
	use crate::integration::utils::{CliTest, assert_output_contains, assert_success};

	#[test]
	fn test_generate_list_templates() {
		let cli = CliTest::new();

		let output = cli.run_command(&["generate", "--list"]);

		assert_success(&output);
		assert_output_contains(&output, "Available Project Templates");
		assert_output_contains(&output, "Basic Neo dApp");
		assert_output_contains(&output, "NEP-17 Token");
	}
}
