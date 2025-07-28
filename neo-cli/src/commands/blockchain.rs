#![allow(dead_code)]
use crate::{
	errors::CliError,
	utils::{extensions::TransactionExtensions, print_error, print_info, print_success},
};
use clap::{Args, Subcommand};
use hex;
use neo3::{neo_clients::APITrait, neo_protocol::NeoBlock};
use primitive_types::H160;
use std::{io, io::Write, path::PathBuf, str::FromStr};

#[derive(Args, Debug)]
pub struct BlockchainArgs {
	#[command(subcommand)]
	pub command: BlockchainCommands,
}

#[derive(Subcommand, Debug)]
pub enum BlockchainCommands {
	/// Export blockchain data
	Export {
		/// Path to save the exported data
		#[arg(short, long)]
		path: PathBuf,

		/// Start block index (inclusive)
		#[arg(short, long, default_value = "0")]
		start: u32,

		/// End block index (inclusive, if not specified, exports to the latest block)
		#[arg(short, long)]
		end: Option<u32>,
	},

	/// Show block details
	ShowBlock {
		/// Block hash or index
		#[arg(short, long)]
		identifier: String,
	},

	/// Show transaction information
	ShowTx {
		/// Transaction hash
		#[arg(short, long)]
		hash: String,
	},

	/// Show contract details
	ShowContract {
		/// Contract hash or script hash
		#[arg(short, long)]
		hash: String,
	},
}

/// CLI state is defined in wallet.rs
#[allow(dead_code)]
pub async fn handle_blockchain_command(
	args: BlockchainArgs,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	match args.command {
		BlockchainCommands::Export { path, start, end } => {
			export_blockchain(path, start, end, state).await
		},
		BlockchainCommands::ShowBlock { identifier } => show_block(identifier, state).await,
		BlockchainCommands::ShowTx { hash } => show_transaction(hash, state).await,
		BlockchainCommands::ShowContract { hash } => show_contract(hash, state).await,
	}
}

#[allow(dead_code)]
async fn export_blockchain(
	path: PathBuf,
	start: u32,
	end: Option<u32>,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!(
		"Exporting blockchain data from block {} to {}...",
		start,
		end.map_or("latest".to_string(), |e| e.to_string())
	));

	let rpc_client = state.rpc_client.as_ref().unwrap();
	let latest_block = rpc_client
		.get_block_count()
		.await
		.map_err(|e| CliError::Network(format!("Failed to get block count: {}", e)))?;
	let end_block = end.unwrap_or(latest_block - 1);

	if start > end_block {
		print_error("Start block index is greater than end block index");
		return Err(CliError::Input("Invalid block range".to_string()));
	}

	// Create export directory if it doesn't exist
	std::fs::create_dir_all(&path).map_err(|e| CliError::Io(e))?;

	// Export blocks
	let mut exported = 0;
	let total_blocks = end_block - start + 1;

	for i in start..=end_block {
		print!("\rExporting block {} of {}...", i, end_block);
		io::stdout().flush().map_err(|e| CliError::Io(e))?;

		let block = match rpc_client.get_block_by_index(i, true).await {
			Ok(block) => block,
			Err(e) => {
				print_error(&format!("Failed to retrieve block {}: {}", i, e));
				continue;
			},
		};

		// Export block to JSON file
		let block_path = path.join(format!("block_{}.json", i));
		let json = serde_json::to_string_pretty(&block)
			.map_err(|e| CliError::Input(format!("Failed to serialize block: {}", e)))?;

		std::fs::write(&block_path, json).map_err(|e| CliError::Io(e))?;

		exported += 1;

		// Show progress
		if exported % 100 == 0 || exported == total_blocks {
			print_info(&format!(
				"Exported {}/{} blocks ({}%)",
				exported,
				total_blocks,
				(exported * 100) / total_blocks
			));
		}
	}

	print_success(&format!("Blockchain data exported to: {:?} ({} blocks)", path, exported));
	Ok(())
}

#[allow(dead_code)]
async fn show_block(
	identifier: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching block: {}", identifier));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	if identifier.starts_with("0x")
		|| (identifier.len() == 64 && identifier.chars().all(|c| c.is_ascii_hexdigit()))
	{
		// Identifier is a hash
		match rpc_client.get_block_by_hash(&identifier, true).await {
			Ok(block) => show_block_by_hash(block),
			Err(e) => {
				print_error(&format!("Failed to get block by hash: {}", e));
				Err(CliError::Network(format!("Failed to get block by hash: {}", e)))
			},
		}
	} else {
		// Try to parse as a block index (integer)
		let index = identifier.parse::<u32>().map_err(|_| {
			CliError::Input(format!(
				"Invalid block identifier. Must be a block hash or block index: {}",
				identifier
			))
		})?;

		show_block_by_index(index, state).await
	}
}

#[allow(dead_code)]
async fn show_block_by_index(
	index: u32,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching block at index: {}", index));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Get block by index
	let block = match rpc_client.get_block_by_index(index, true).await {
		Ok(block) => block,
		Err(e) => return Err(CliError::Rpc(format!("Failed to get block by index: {}", e))),
	};

	// Display block information
	println!("Block Hash: {}", block.hash);
	println!("Block Index: {}", block.index);
	println!("Block Time: {}", block.time);
	println!("Block Size: {}", block.size);
	println!("Transaction Count: {}", block.transactions.as_ref().map_or(0, |tx| tx.len()));
	println!("Merkle Root: {}", block.merkle_root_hash);
	println!("Previous Block: {}", block.prev_block_hash);
	println!("Next Consensus: {}", block.next_consensus);

	// Show transactions if there are any
	if let Some(transactions) = &block.transactions {
		if !transactions.is_empty() {
			println!("\nTransactions:");
			for (i, tx) in transactions.iter().enumerate() {
				println!("  {}. Hash: {}", i + 1, tx.hash);
			}
		}
	}

	print_success("Block information retrieved successfully");
	Ok(())
}

#[allow(dead_code)]
fn show_block_by_hash(block: NeoBlock) -> Result<(), CliError> {
	// Display block information
	println!("Block Hash: {}", block.hash);
	println!("Block Index: {}", block.index);
	println!("Block Time: {}", block.time);
	println!("Block Size: {}", block.size);
	println!("Transaction Count: {}", block.transactions.as_ref().map_or(0, |tx| tx.len()));
	println!("Merkle Root: {}", block.merkle_root_hash);
	println!("Previous Block: {}", block.prev_block_hash);
	println!("Next Consensus: {}", block.next_consensus);

	// Show transactions if there are any
	if let Some(transactions) = &block.transactions {
		if !transactions.is_empty() {
			println!("\nTransactions:");
			for (i, tx) in transactions.iter().enumerate() {
				println!("  {}. Hash: {}", i + 1, tx.hash);
			}
		}
	}

	print_success("Block information retrieved successfully");
	Ok(())
}

#[allow(dead_code)]
async fn show_transaction(
	hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching transaction information for: {}", hash));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Remove '0x' prefix if present
	let hash_str = hash.strip_prefix("0x").unwrap_or(&hash);

	// Convert to H256
	let hash_bytes = hex::decode(hash_str)
		.map_err(|e| CliError::Input(format!("Invalid transaction hash format: {}", e)))?;

	if hash_bytes.len() != 32 {
		return Err(CliError::Input("Transaction hash must be 32 bytes".to_string()));
	}

	let hash = primitive_types::H256::from_slice(&hash_bytes);

	let tx = rpc_client
		.get_transaction(hash)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to retrieve transaction: {}", e)))?;

	// Display transaction information
	println!("Transaction Hash: {}", tx.hash);
	println!("Transaction Type: {}", tx.type_name());
	println!("Transaction Size: {}", tx.size);
	println!("Transaction Version: {}", tx.version);
	println!("Transaction Nonce: {}", tx.nonce);
	println!("Transaction Sender: {}", tx.sender);
	println!("Transaction System Fee: {}", tx.sys_fee);
	println!("Transaction Network Fee: {}", tx.net_fee);
	println!("Transaction Valid Until Block: {}", tx.valid_until_block);

	// Display signers
	println!("\nTransaction Signers ({}):", tx.signers.len());
	for (i, signer) in tx.signers.iter().enumerate() {
		println!("  {}. Account: {}", i + 1, signer.account);
		println!("     Scopes: {:?}", signer.scopes);
		if !signer.allowed_contracts.is_empty() {
			println!("     Allowed Contracts: {:?}", signer.allowed_contracts);
		}
		if !signer.allowed_groups.is_empty() {
			println!("     Allowed Groups: {:?}", signer.allowed_groups);
		}
	}

	// Show witnesses if any
	if !tx.witnesses.is_empty() {
		println!("\nWitnesses ({}):", tx.witnesses.len());
		for (i, witness) in tx.witnesses.iter().enumerate() {
			println!("  {}. Invocation Script: 0x{}", i + 1, hex::encode(&witness.invocation));
			println!("     Verification Script: 0x{}", hex::encode(&witness.verification));
		}
	}

	// Display script
	println!("\nTransaction Script: 0x{}", hex::encode(&tx.script));

	print_success("Transaction information retrieved successfully");
	Ok(())
}

#[allow(dead_code)]
async fn show_contract(
	hash: String,
	state: &mut crate::commands::wallet::CliState,
) -> Result<(), CliError> {
	if state.rpc_client.is_none() {
		print_error("No RPC client is connected. Please connect to a node first.");
		return Err(CliError::Network("No RPC client is connected".to_string()));
	}

	print_info(&format!("Fetching contract information for: {}", hash));

	let rpc_client = state.rpc_client.as_ref().unwrap();

	// Convert from string to H160
	let contract_hash = H160::from_str(&hash)
		.map_err(|_| CliError::Input(format!("Invalid contract hash format: {}", hash)))?;

	let contract = rpc_client
		.get_contract_state(contract_hash)
		.await
		.map_err(|e| CliError::Rpc(format!("Failed to retrieve contract: {}", e)))?;

	// Display contract information
	println!("Contract Hash: {}", contract.hash);
	println!("Contract ID: {}", contract.id);
	println!("Update Counter: {}", contract.update_counter);

	println!("\nManifest:");
	println!("  Name: {:?}", contract.manifest.name);
	println!("  Groups: {:?}", contract.manifest.groups);
	println!("  Features: {:?}", contract.manifest.features);
	println!("  Supported Standards: {:?}", contract.manifest.supported_standards);
	println!("  Trusts: {:?}", contract.manifest.trusts);

	if let Some(abi) = &contract.manifest.abi {
		println!("\n  ABI Methods ({}):", abi.methods.len());
		for (i, method) in abi.methods.iter().enumerate() {
			println!("    {}. {} ({} parameters)", i + 1, method.name, method.parameters.len());
		}

		println!("\n  ABI Events ({}):", abi.events.len());
		for (i, event) in abi.events.iter().enumerate() {
			println!("    {}. {} ({} parameters)", i + 1, event.name, event.parameters.len());
		}
	} else {
		println!("\n  No ABI available");
	}

	println!("\nPermissions ({}):", contract.manifest.permissions.len());
	for (i, perm) in contract.manifest.permissions.iter().enumerate() {
		println!("    {}. {:?}", i + 1, perm);
	}

	print_success("Contract information retrieved successfully");
	Ok(())
}
