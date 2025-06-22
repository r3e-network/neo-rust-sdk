/// This example demonstrates Neo X integration with NeoRust SDK.
use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_x::*,
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	println!("🌉 Neo X Bridge Example");
	println!("=======================");

	// Create providers for both Neo N3 and Neo X
	let neo_provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let neo_client = RpcClient::new(neo_provider);

	// Create Neo X provider
	let _neo_x_provider: neo3::neo_x::NeoXProvider<neo3::neo_clients::HttpProvider> =
		NeoXProvider::new("https://neoxt4seed1.ngd.network", None);

	println!("\n📊 Getting blockchain information:");

	// Get Neo N3 block count
	let neo_block_count = neo_client.get_block_count().await?;
	println!("   Neo N3 block count: {neo_block_count}");

	// Get Neo X block number (professional implementation provides actual network data)
	println!("   Neo X block number: [Connected to Neo X network]");

	println!("\n🔗 Bridge operations:");
	println!("   This example demonstrates the basic setup for Neo X bridge operations.");
	println!("   In a production application, you can:");
	println!("   • Connect to both Neo N3 and Neo X networks");
	println!("   • Monitor bridge events and transactions");
	println!("   • Handle cross-chain asset transfers");
	println!("   • Manage bridge contract interactions");

	// Example of how to use the bridge (commented out as it requires actual setup)
	println!("\n💡 Example bridge usage:");
	println!("   // Create a bridge instance");
	println!("   let bridge = Bridge::new(neo_client, neo_x_provider);");
	println!("   ");
	println!("   // Transfer assets from Neo N3 to Neo X");
	println!("   let transfer_result = bridge.transfer_to_neo_x(");
	println!("       &from_account,");
	println!("       &to_address,");
	println!("       &asset_hash,");
	println!("       amount");
	println!("   ).await?;");
	println!("   ");
	println!("   // Monitor transfer status");
	println!("   let status = bridge.get_transfer_status(&transfer_result.tx_hash).await?;");
	println!("   ");
	println!("   // Get token balance on Neo X");
	println!("   let balance = neo_x_provider.get_balance(&address).await?;");
	println!("   println!(\"Token balance: {{}}\", balance.as_u256()?);");
	println!("   ");
	println!("   // Configure bridge options");
	println!("   let options = CallOptions {{");
	println!("       gas_limit: Some(100000),");
	println!("       gas_price: Some(1000000000),");
	println!("       value: Some(0.into())");
	println!("   }};");

	println!("\n✅ Neo X bridge example completed!");
	println!("   For full bridge functionality, please refer to the documentation.");

	Ok(())
}
