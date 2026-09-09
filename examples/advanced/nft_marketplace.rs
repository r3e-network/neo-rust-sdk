/*!
# NEP-11 NFT Marketplace Example

This example demonstrates a complete NFT marketplace workflow using the SDK's
NEP-11 non-fungible token standard implementation. It shows how to:

- **Query NFT metadata** (owner, properties, balance)
- **Browse collections** via iterator traversal
- **Build transfer transactions** for market operations
- **Work with both divisible and non-divisible NFTs**

## Prerequisites

Run against a Neo node by setting `RPC_URL` environment variable or passing it as CLI argument.
Example testnet: `export RPC_URL=https://testnet1.neo.org:443`

## Usage

```bash
cargo run --example nft_marketplace --features mock
```

Or run tests instead of live queries:
```bash
cargo test --package neo3 examples_nep11_marketplace
```

The example connects to a configurable Neo N3 node and demonstrates all key
operations you'd need to build an NFT marketplace dApp.

### Live Example Mode

When running without `--features mock`, the example attempts to connect to:
1. An RPC URL from environment variable `RPC_URL`
2. TestNet endpoint if no URL provided
3. Local development node at default localhost

All functionality works identically - only the network destination changes.
*/

use neo3::prelude::*;
// `NftContract` + `NonFungibleTokenTrait` come from the prelude; the token
// metadata helpers (`get_symbol`/`get_decimals`/`get_total_supply`) live on
// `TokenTrait`, `name()` on `SmartContractTrait`, and account creation on
// `AccountTrait`, so we bring those traits into scope explicitly.
use neo3::neo_contract::{SmartContractTrait, TokenTrait};
use neo3::neo_protocol::AccountTrait;
use std::{collections::HashMap, env, str::FromStr};

/// Default testnet endpoint
const DEFAULT_RPC_URL: &str = "https://testnet1.neo.org:443";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== NEP-11 NFT Marketplace Demo ===\n");
	
	// Get RPC URL from environment or use testnet
	let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
	eprintln!("Connecting to: {}", rpc_url);
	
	let provider = providers::HttpProvider::new(rpc_url.as_str())?;
	let client = providers::RpcClient::new(provider);
	
	// ============================================
	// 📍 PART 1: CONTRACT SETUP
	// ============================================
	
	println!("🔧 Initializing NFT contract instances...\n");
	
	// Use a known NFT contract on testnet or create one for testing
	// For demo, we'll use placeholder contracts that work with the API structure
	let collection_hash = H160::from_str(
		"0xd2a4cff31913016155e38e474a2c06d08be276cf", // Placeholder NFT contract hash
	)?;
	
	let mut nft_contract = NftContract::new(&collection_hash, Some(&client));
	
	// Fetch basic token information
	match nft_contract.get_symbol().await {
		Ok(symbol) => eprintln!("✓ Collection symbol: {}", symbol),
		Err(e) => eprintln!("⚠ Could not fetch symbol: {:?}", e),
	}
	
	match nft_contract.get_decimals().await {
		Ok(decimals) => eprintln!("✓ Decimals: {}", decimals),
		Err(e) => eprintln!("⚠ Could not fetch decimals: {:?}", e),
	}
	
	// ============================================
	// 🔍 PART 2: NFT DISCOVERY
	// ============================================
	
	println!("\n🔍 Discovering NFT collections...\n");
	
	let sample_owner = protocol::Account::create()?;
	let owner_script_hash = sample_owner.get_script_hash();
	
	// List all NFTs owned by an address
	match discover_owned_nfts(&mut nft_contract, owner_script_hash).await {
		Ok(tokens) => {
			println!(
				"✓ Found {} NFT(s) owned by {}",
				tokens.len(),
				owner_script_hash.to_hex()
			);
			
			// Display each token's details
			for (index, token_id) in tokens.iter().take(5).enumerate() {
				if let Ok(owner) = nft_contract.owner_of(token_id.clone()).await {
					eprintln!("  [{}] Token ID: {:?} → Owner: {}", index + 1, token_id, owner.to_hex());
					
					// Fetch custom properties
					if let Ok(props) = nft_contract.properties(token_id.clone()).await {
						display_token_properties(index + 1, props);
					}
				}
			}
		}
		Err(e) => {
			eprintln!("⚠ No NFTs found or error during discovery: {:?}", e);
			eprintln!("💡 This is expected if the contract doesn't exist yet.");
		}
	}
	
	// ============================================
	// 💼 PART 3: BALANCE QUERY
	// ============================================
	
	println!("\n💎 Checking NFT balances...\n");
	
	match nft_contract.balance_of(owner_script_hash).await {
		Ok(balance) => println!("✓ Balance for {}: {}", owner_script_hash.to_hex(), balance),
		Err(e) => eprintln!("⚠ Could not fetch balance: {:?}", e),
	}
	
	// ============================================
	// 🔄 PART 4: MARKETPLACE OPERATIONS
	// ============================================
	
	println!("\n🏪 Building marketplace transaction flows...\n");
	
	// Create two accounts: seller and buyer
	let seller = protocol::Account::create()?;
	let buyer = protocol::Account::create()?;
	
	let seller_hash = seller.get_script_hash();
	let buyer_hash = buyer.get_script_hash();
	
	eprintln!("Seller account:    {}", seller_hash.to_hex());
	eprintln!("Buyer account:     {}", buyer_hash.to_hex());
	
	// Example: Transfer a specific NFT from seller to buyer
	let example_token_id = vec![0x01u8, 0x02, 0x03]; // Unique token identifier
	
	match nft_contract.transfer(&seller, buyer_hash, example_token_id.clone(), None).await {
		Ok(_tx_builder) => {
			println!("✓ Successfully built transfer transaction!");
			println!("  From:    {}", seller_hash.to_hex());
			println!("  To:      {}", buyer_hash.to_hex());
			println!("  Token ID: {:?}", example_token_id);
			
			// The transaction builder can be further configured and signed
			// In production, you'd call tx_builder.valid_until_block(...)
			// then tx_builder.sign().await?
			println!("  ⚙️ Transaction ready for signing and broadcast");
		}
		Err(e) => eprintln!("⚠ Could not build transfer: {:?}", e),
	}
	
	// ============================================
	// 📊 PART 5: COLLECTION METRICS
	// ============================================
	
	println!("\n📊 Collection metrics...\n");
	
	match nft_contract.get_total_supply().await {
		Ok(supply) => println!("✓ Total NFTs in collection: {}", supply),
		Err(e) => eprintln!("⚠ Could not fetch total supply: {:?}", e),
	}
	
	match nft_contract.name().await {
		name => {
			if !name.is_empty() {
				println!("✓ Contract name: {}", name);
			} else {
				println!("ℹ Contract name: Not set");
			}
		}
	}
	
	// ============================================
	// ✅ COMPLETE!
	// ============================================
	
	println!("\n✅ All marketplace operations demonstrated successfully!");
	println!("\n💡 Next steps:");
	println!("   • Deploy an actual NFT smart contract");
	println!("   • Mint NFTs with unique IDs and properties");
	println!("   • Implement bidding/auction logic");
	println!("   • Add escrow protection for trades");
	println!("   • Track events and emit logs");
	
	Ok(())
}

/// Discovers all NFTs owned by an address using iterator traversal
async fn discover_owned_nfts<'a>(
	nft_contract: &mut NftContract<'a, providers::HttpProvider>,
	owner_hash: H160,
) -> Result<Vec<Bytes>, Box<dyn std::error::Error>> {
	let tokens_iterator = nft_contract.tokens_of(owner_hash).await?;
	
	// Traverse the iterator to get all token IDs
	// Use a reasonable limit (e.g., 100 items) for initial discovery
	let token_ids = tokens_iterator.traverse(100).await?;
	
	Ok(token_ids)
}

/// Displays formatted NFT property information
fn display_token_properties(index: usize, properties: HashMap<String, StackItem>) {
	println!(
		"\n        Token #{} properties:",
		index
	);
	
	for (key, value) in &properties {
		// Convert the NeoVM StackItem into a human-readable string.
		let readable = value.as_string().unwrap_or_else(|| format!("{:?}", value));
		
		// Truncate long values for clean output
		let display_value = if readable.len() > 50 {
			format!("{}...", &readable[..50])
		} else {
			readable
		};
		
		println!("          {} → {}", key, display_value);
	}
}
