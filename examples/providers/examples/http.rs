//! The Http transport is used to send JSON-RPC requests over HTTP to an Neo node.
//! This is the most basic connection to a node.

use neo3::neo_clients::APITrait;
use std::sync::Arc;

const RPC_URL: &str = "https://testnet1.neo.org:443/";

#[tokio::main]
async fn main() -> eyre::Result<()> {
	create_instance().await?;
	share_providers_across_tasks().await?;
	Ok(())
}

async fn create_instance() -> eyre::Result<()> {
	// Create an HTTP provider for Neo N3 TestNet
	let provider = neo3::neo_clients::HttpProvider::new(RPC_URL)?;
	let client = neo3::neo_clients::RpcClient::new(provider);

	// The client can be used to make RPC calls
	let block_count = client.get_block_count().await?;
	println!("Current block count: {block_count}");

	// Get the latest block hash
	let latest_block_hash = client.get_block_hash(block_count - 1).await?;
	println!("Latest block hash: {latest_block_hash:?}");

	Ok(())
}

/// Providers can be easily shared across tasks using `Arc` smart pointers
async fn share_providers_across_tasks() -> eyre::Result<()> {
	let provider = neo3::neo_clients::HttpProvider::new(RPC_URL)?;
	let client = neo3::neo_clients::RpcClient::new(provider);

	let client_1 = Arc::new(client);
	let client_2 = Arc::clone(&client_1);

	let handle1 = tokio::spawn(async move { client_1.get_block_count().await.unwrap_or(0) });

	let handle2 = tokio::spawn(async move { client_2.get_block_count().await.unwrap_or(0) });

	let block1: u32 = handle1.await?;
	let block2: u32 = handle2.await?;

	println!("Block count from client 1: {block1}");
	println!("Block count from client 2: {block2}");

	Ok(())
}
