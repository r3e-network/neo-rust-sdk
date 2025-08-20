use crate::{
	neo_builder::{Signer, TransactionError},
	neo_clients::{APITrait, ProviderError},
};

/// Enhanced gas estimation utilities for Neo N3 transactions
pub struct GasEstimator;

impl GasEstimator {
	/// Estimate gas consumption using real-time invokescript RPC call
	///
	/// This provides precise gas calculation by actually executing the script
	/// on the blockchain without committing the transaction.
	///
	/// # Arguments
	/// * `client` - The RPC client connected to a Neo node
	/// * `script` - The script bytes to estimate gas for
	/// * `signers` - The transaction signers
	///
	/// # Returns
	/// The estimated gas consumption in GAS units
	pub async fn estimate_gas_realtime<T>(
		client: &T,
		script: &[u8],
		signers: Vec<Signer>,
	) -> Result<i64, TransactionError>
	where
		T: APITrait,
		T::Error: Into<ProviderError>,
	{
		// Convert script to hex string
		let script_hex = hex::encode(script);

		// Call invokescript RPC method for real-time gas calculation
		let result = client
			.invoke_script(script_hex, signers)
			.await
			.map_err(|e| TransactionError::ProviderError(e.into()))?;

		// Check if execution was successful
		if result.has_state_fault() {
			return Err(TransactionError::TransactionConfiguration(format!(
				"Script execution failed: {}",
				result.exception.unwrap_or_else(|| "Unknown error".to_string())
			)));
		}

		// Parse and return gas consumed
		let gas_consumed = result.gas_consumed.parse::<i64>().map_err(|_| {
			TransactionError::IllegalState("Failed to parse gas consumed".to_string())
		})?;

		Ok(gas_consumed)
	}

	/// Estimate gas with safety margin
	///
	/// Adds a configurable safety margin to the estimated gas to account for
	/// network conditions and small variations in execution.
	///
	/// # Arguments
	/// * `client` - The RPC client
	/// * `script` - The script to estimate
	/// * `signers` - Transaction signers
	/// * `margin_percent` - Safety margin as percentage (e.g., 10 for 10% extra)
	///
	/// # Returns
	/// The estimated gas with safety margin applied
	pub async fn estimate_gas_with_margin<T>(
		client: &T,
		script: &[u8],
		signers: Vec<Signer>,
		margin_percent: u8,
	) -> Result<i64, TransactionError>
	where
		T: APITrait,
		T::Error: Into<ProviderError>,
	{
		let base_gas = Self::estimate_gas_realtime(client, script, signers).await?;
		let margin = (base_gas as f64 * (margin_percent as f64 / 100.0)) as i64;
		Ok(base_gas + margin)
	}

	/// Batch estimate gas for multiple scripts
	///
	/// Efficiently estimates gas for multiple scripts in parallel.
	///
	/// # Arguments
	/// * `client` - The RPC client
	/// * `scripts` - Vector of scripts with their signers
	///
	/// # Returns
	/// Vector of gas estimates corresponding to each script
	pub async fn batch_estimate_gas<T>(
		client: &T,
		scripts: Vec<(&[u8], Vec<Signer>)>,
	) -> Result<Vec<i64>, TransactionError>
	where
		T: APITrait + Clone + 'static,
		T::Error: Into<ProviderError>,
	{
		use futures::future::join_all;

		let futures = scripts.into_iter().map(|(script, signers)| {
			let client_clone = client.clone();
			async move { Self::estimate_gas_realtime(&client_clone, script, signers).await }
		});

		let results = join_all(futures).await;

		// Collect results, propagating any errors
		let mut estimates = Vec::new();
		for result in results {
			estimates.push(result?);
		}

		Ok(estimates)
	}

	/// Compare estimated gas with actual gas after execution
	///
	/// Useful for calibrating estimation accuracy and adjusting safety margins.
	///
	/// # Arguments
	/// * `estimated` - The estimated gas consumption
	/// * `actual` - The actual gas consumed after execution
	///
	/// # Returns
	/// The percentage difference between estimated and actual
	pub fn calculate_estimation_accuracy(estimated: i64, actual: i64) -> f64 {
		if actual == 0 {
			return 0.0;
		}

		let diff = (estimated - actual).abs() as f64;
		(diff / actual as f64) * 100.0
	}
}

/// Extension trait for TransactionBuilder to add real-time gas estimation
pub trait TransactionBuilderGasExt {
	/// Estimate gas using real-time invokescript
	async fn estimate_gas_realtime(&self) -> Result<i64, TransactionError>;

	/// Estimate gas with safety margin
	async fn estimate_gas_with_margin(&self, margin_percent: u8) -> Result<i64, TransactionError>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_calculate_estimation_accuracy() {
		// Perfect estimate
		assert_eq!(GasEstimator::calculate_estimation_accuracy(100, 100), 0.0);

		// 10% overestimate
		assert_eq!(GasEstimator::calculate_estimation_accuracy(110, 100), 10.0);

		// 10% underestimate
		assert_eq!(GasEstimator::calculate_estimation_accuracy(90, 100), 10.0);

		// Edge case: actual is 0
		assert_eq!(GasEstimator::calculate_estimation_accuracy(100, 0), 0.0);
	}
}
