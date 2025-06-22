use neo3::neo_clients::{APITrait, HttpProvider, RpcClient};

/// This example demonstrates how to build a simple block explorer to view block and transaction details.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Neo N3 Block Explorer Example");
	println!("============================");

	// Connect to Neo N3 TestNet
	println!("\n1. Connecting to Neo N3 TestNet...");
	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);
	println!("Connected to TestNet");

	// Get the current block height
	println!("\n2. Getting block information...");
	let block_count = client.get_block_count().await?;
	println!("Current block count: {block_count}");

	// Get the latest block
	let latest_block_index = block_count - 1;
	println!("\n3. Retrieving latest block (index {latest_block_index})...");
	let latest_block = client.get_block_by_index(latest_block_index, true).await?;

	println!("Block hash: {}", latest_block.hash);
	println!("Block size: {} bytes", latest_block.size);
	println!("Block time: {}", latest_block.time);
	println!("Block version: {}", latest_block.version);
	println!("Previous block: {}", latest_block.prev_block_hash);
	println!("Merkle root: {}", latest_block.merkle_root_hash);
	println!(
		"Transaction count: {}",
		latest_block.transactions.as_ref().map_or(0, |txs| txs.len())
	);

	// Display transaction information
	println!("\n4. Examining transactions in this block...");
	if let Some(transactions) = &latest_block.transactions {
		for (i, tx) in transactions.iter().take(3).enumerate() {
			println!("\nTransaction #{}", i + 1);
			println!("Hash: {}", tx.hash);
			println!("Size: {} bytes", tx.size);
			println!("Version: {}", tx.version);
			println!("Nonce: {}", tx.nonce);
			println!("Sender: {}", tx.sender);
			println!("System fee: {}", tx.sys_fee);
			println!("Network fee: {}", tx.net_fee);
			println!("Valid until block: {}", tx.valid_until_block);
		}

		// If there are more than 3 transactions, show a message
		if transactions.len() > 3 {
			println!("\n... and {} more transactions", transactions.len() - 3);
		}
	} else {
		println!("No transactions in this block");
	}

	// Get network information
	println!("\n5. Retrieving network information...");
	let peers = client.get_peers().await?;
	println!("Connected peers: {}", peers.connected.len());
	println!("Unconnected peers: {}", peers.unconnected.len());
	println!("Bad peers: {}", peers.bad.len());

	println!("\nBlock explorer example completed successfully!");
	Ok(())
}
