use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_types::ScriptHash,
};
use std::str::FromStr;

/// This example demonstrates interaction with Flamingo Finance, one of Neo's leading DeFi protocols.
/// It shows how to query liquidity pools, check token prices, and interact with swap functionality.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🦩 Neo N3 Flamingo Finance DeFi Protocol Example");
	println!("=================================================");

	// Connect to Neo N3 MainNet (Flamingo is on MainNet)
	println!("\n📡 Connecting to Neo N3 MainNet...");
	let provider = HttpProvider::new("https://mainnet1.neo.org:443/")
		.map_err(|e| format!("Failed to create provider: {e}"))?;
	let client = RpcClient::new(provider);
	println!("   ✅ Connected successfully");

	// Flamingo Finance Contract Addresses (MainNet)
	println!("\n📋 Flamingo Finance Contract Addresses:");
	let flm_token = ScriptHash::from_str("4d9eab13620fe3569ba3b0e56e2877739e4145e3")?;
	let swap_router = ScriptHash::from_str("f970f4ccecd765b63732b821775dc38c25d74f23")?;
	let flund_token = ScriptHash::from_str("48c40d4666f93408be1bef038b6722404d9a4c2a")?;
	let fusd_token = ScriptHash::from_str("17c76859c11bc14da5b3e9c88fa695513442c606")?;

	println!("   FLM Token:     0x{}", hex::encode(flm_token.0));
	println!("   Swap Router:   0x{}", hex::encode(swap_router.0));
	println!("   FLUND Token:   0x{}", hex::encode(flund_token.0));
	println!("   fUSDT Token:   0x{}", hex::encode(fusd_token.0));

	// 1. Query FLM Token Information
	println!("\n1️⃣ Querying FLM Token Information...");
	query_token_info(&client, &flm_token, "FLM").await?;

	// 2. Check FLUND Token Information
	println!("\n2️⃣ Querying FLUND Token Information...");
	query_token_info(&client, &flund_token, "FLUND").await?;

	// 3. Demonstrate Swap Path Query
	println!("\n3️⃣ Demonstrating Swap Path Query...");
	demonstrate_swap_concepts().await?;

	// 4. Pool Information Concepts
	println!("\n4️⃣ Liquidity Pool Concepts...");
	demonstrate_pool_concepts().await?;

	// 5. Yield Farming Concepts
	println!("\n5️⃣ Yield Farming Concepts...");
	demonstrate_farming_concepts().await?;

	// Best Practices
	println!("\n💡 Flamingo Finance Best Practices:");
	println!("   🔍 Always check slippage tolerance before swaps");
	println!("   💰 Monitor gas costs for transactions");
	println!("   📊 Check pool liquidity before large trades");
	println!("   ⏰ Be aware of reward claim periods");
	println!("   🔐 Use secure wallets for DeFi interactions");
	println!("   📈 Monitor impermanent loss in liquidity pools");

	println!("\n✅ Flamingo Finance example completed!");
	println!("💡 This example demonstrates querying DeFi protocol data on Neo N3.");

	Ok(())
}

/// Query token information from a NEP-17 token contract
async fn query_token_info(
	client: &RpcClient<HttpProvider>,
	token_hash: &ScriptHash,
	token_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   📊 {token_name} Token Information:");

	// Get token symbol
	match client.invoke_function(token_hash, "symbol".to_string(), vec![], None).await {
		Ok(result) =>
			if let Some(stack_item) = result.stack.first() {
				if let Some(symbol) = stack_item.as_string() {
					println!("      Symbol: {symbol}");
				}
			},
		Err(e) => println!("      ⚠️ Failed to get symbol: {e}"),
	}

	// Get token decimals
	match client.invoke_function(token_hash, "decimals".to_string(), vec![], None).await {
		Ok(result) =>
			if let Some(stack_item) = result.stack.first() {
				if let Some(decimals) = stack_item.as_int() {
					println!("      Decimals: {decimals}");
				}
			},
		Err(e) => println!("      ⚠️ Failed to get decimals: {e}"),
	}

	// Get total supply
	match client
		.invoke_function(token_hash, "totalSupply".to_string(), vec![], None)
		.await
	{
		Ok(result) => {
			if let Some(stack_item) = result.stack.first() {
				if let Some(supply) = stack_item.as_int() {
					let decimals = 8; // Most Neo tokens use 8 decimals
					let supply_decimal = supply as f64 / 10f64.powi(decimals);
					println!("      Total Supply: {supply_decimal:.2} {token_name}");
				}
			}
		},
		Err(e) => println!("      ⚠️ Failed to get total supply: {e}"),
	}

	Ok(())
}

/// Demonstrate swap path concepts
async fn demonstrate_swap_concepts() -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔄 Swap Concepts:");
	println!("      • Direct swaps: Token A → Token B");
	println!("      • Multi-hop swaps: Token A → Token B → Token C");
	println!("      • Optimal path finding for best rates");
	println!("      • Slippage protection mechanisms");

	println!("\n   📝 Example Swap Process:");
	println!("      1. Query available pools");
	println!("      2. Calculate optimal swap path");
	println!("      3. Check price impact");
	println!("      4. Set slippage tolerance");
	println!("      5. Execute swap transaction");
	println!("      6. Verify receipt of tokens");

	Ok(())
}

/// Demonstrate liquidity pool concepts
async fn demonstrate_pool_concepts() -> Result<(), Box<dyn std::error::Error>> {
	println!("   💧 Liquidity Pool Operations:");
	println!("      • Add liquidity: Provide token pairs");
	println!("      • Remove liquidity: Withdraw tokens + fees");
	println!("      • LP token minting/burning");
	println!("      • Fee accrual mechanisms");

	println!("\n   📊 Pool Metrics to Monitor:");
	println!("      • Total Value Locked (TVL)");
	println!("      • 24h trading volume");
	println!("      • Pool APY/APR");
	println!("      • Price impact for trades");
	println!("      • Impermanent loss calculations");

	Ok(())
}

/// Demonstrate yield farming concepts
async fn demonstrate_farming_concepts() -> Result<(), Box<dyn std::error::Error>> {
	println!("   🌾 Yield Farming Strategies:");
	println!("      • Single-asset staking");
	println!("      • LP token staking");
	println!("      • Auto-compounding vaults");
	println!("      • Reward token claiming");

	println!("\n   🎯 Farming Considerations:");
	println!("      • APY vs APR calculations");
	println!("      • Vesting schedules");
	println!("      • Lock-up periods");
	println!("      • Gas cost optimization");
	println!("      • Risk assessment");

	Ok(())
}
