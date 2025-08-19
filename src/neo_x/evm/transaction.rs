use primitive_types::H160;
use serde::{Deserialize, Serialize};

/// Neo X EVM transaction for interacting with the Neo X EVM-compatible chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoXTransaction {
	to: Option<H160>,
	data: Vec<u8>,
	value: u64,
	gas_limit: u64,
	gas_price: u64,
}

impl NeoXTransaction {
	/// Creates a new NeoXTransaction instance
	///
	/// # Arguments
	///
	/// * `to` - The recipient address (None for contract creation)
	/// * `data` - The transaction data
	/// * `value` - The transaction value
	/// * `gas_limit` - The gas limit for the transaction
	/// * `gas_price` - The gas price for the transaction
	///
	/// # Returns
	///
	/// A new NeoXTransaction instance
	pub fn new(
		to: Option<H160>,
		data: Vec<u8>,
		value: u64,
		gas_limit: u64,
		gas_price: u64,
	) -> Self {
		Self { to, data, value, gas_limit, gas_price }
	}

	/// Gets the recipient address
	///
	/// # Returns
	///
	/// The recipient address as an Option<H160>
	pub fn to(&self) -> Option<H160> {
		self.to
	}

	/// Gets the transaction data
	///
	/// # Returns
	///
	/// The transaction data as a Vec<u8>
	pub fn data(&self) -> &Vec<u8> {
		&self.data
	}

	/// Gets the transaction value
	///
	/// # Returns
	///
	/// The transaction value as a u64
	pub fn value(&self) -> u64 {
		self.value
	}

	/// Gets the gas limit for the transaction
	///
	/// # Returns
	///
	/// The gas limit as a u64
	pub fn gas_limit(&self) -> u64 {
		self.gas_limit
	}

	/// Gets the gas price for the transaction
	///
	/// # Returns
	///
	/// The gas price as a u64
	pub fn gas_price(&self) -> u64 {
		self.gas_price
	}
}
