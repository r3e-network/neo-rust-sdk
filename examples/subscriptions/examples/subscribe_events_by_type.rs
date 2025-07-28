/// Neo N3 Event Subscription by Type Example
///
/// This example demonstrates how to subscribe to and filter specific types of contract events
/// on the Neo N3 blockchain using polling-based monitoring with event type filtering.
use neo3::neo_clients::APITrait;
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone)]
struct EventFilter {
	contract_hashes: Vec<neo3::neo_types::ScriptHash>,
	event_names: Vec<String>,
	from_block: u32,
}

#[derive(Debug, Clone)]
struct ContractEvent {
	block_height: u32,
	transaction_hash: String,
	contract_hash: neo3::neo_types::ScriptHash,
	event_name: String,
	state: Option<neo3::neo_types::StackItem>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🎯 Neo N3 Event Subscription by Type Example");
	println!("============================================\n");

	// Connect to TestNet
	let client = connect_to_testnet().await?;

	// 1. Subscribe to Transfer events from NEP-17 tokens
	println!("1️⃣ Subscribing to Transfer Events...");
	subscribe_to_transfers(&client).await?;

	// 2. Monitor specific contract events
	println!("\n2️⃣ Monitoring Contract-Specific Events...");
	monitor_contract_specific_events(&client).await?;

	// 3. Multi-event type subscription
	println!("\n3️⃣ Multi-Event Type Subscription...");
	multi_event_subscription(&client).await?;

	// 4. Custom event filtering
	println!("\n4️⃣ Custom Event Filtering...");
	demonstrate_custom_filtering(&client).await?;

	println!("\n✅ Event subscription by type example completed!");
	println!("💡 This demonstrates various event filtering patterns for Neo N3");

	Ok(())
}

async fn connect_to_testnet(
) -> Result<neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>, Box<dyn std::error::Error>>
{
	let endpoints = vec![
		"https://testnet1.neo.org:443/",
		"https://testnet2.neo.org:443/",
		"http://seed1t5.neo.org:20332",
	];

	for endpoint in endpoints {
		if let Ok(provider) = neo3::neo_clients::HttpProvider::new(endpoint) {
			let client = neo3::neo_clients::RpcClient::new(provider);
			if let Ok(height) = client.get_block_count().await {
				println!("   ✅ Connected to: {endpoint}");
				println!("   📦 Block height: {height}\n");
				return Ok(client);
			}
		}
	}

	Err("Failed to connect to TestNet".into())
}

async fn subscribe_to_transfers(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   💸 Monitoring Transfer events from NEP-17 tokens...");

	// Common NEP-17 token contracts on TestNet
	let nep17_contracts = [
		("GAS", "d2a4cff31913016155e38e474a2c06d08be276cf"),
		("NEO", "ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5"),
	];

	let current_height = client.get_block_count().await?;
	let start_height = current_height.saturating_sub(10);

	println!(
		"   📊 Scanning blocks {} to {} for Transfer events",
		start_height,
		current_height - 1
	);

	let mut transfer_events = Vec::new();

	for height in start_height..current_height {
		if let Ok(block) = client.get_block_by_index(height, true).await {
			if let Some(transactions) = &block.transactions {
				for tx in transactions {
					if let Ok(app_log) = client.get_application_log(tx.hash).await {
						for execution in &app_log.executions {
							for notification in &execution.notifications {
								// Filter for Transfer events only
								if notification.event_name == "Transfer" {
									let contract_hex = format!("{:x}", notification.contract);

									// Check if it's from a known NEP-17 contract
									if let Some((name, _)) = nep17_contracts
										.iter()
										.find(|(_, hash)| *hash == contract_hex)
									{
										transfer_events.push(ContractEvent {
											block_height: height,
											transaction_hash: tx.hash.to_string(),
											contract_hash: notification.contract,
											event_name: notification.event_name.clone(),
											state: Some(notification.state.clone()),
										});

										println!("      📢 {name} Transfer found:");
										println!("         Block: {height}");
										println!("         TX: {}", tx.hash);
										{
											let state = &notification.state;
											println!("         State: {state:?}");
										}
									}
								}
							}
						}
					}
				}
			}
		}
	}

	println!("   📈 Transfer Summary:");
	println!("      Total Transfer events found: {}", transfer_events.len());

	Ok(())
}

async fn monitor_contract_specific_events(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🎯 Monitoring events from specific contracts...");

	// Set up filter for GAS contract events
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let filter = EventFilter {
		contract_hashes: vec![gas_hash],
		event_names: vec!["Transfer".to_string(), "Approval".to_string()],
		from_block: client.get_block_count().await?.saturating_sub(5),
	};

	let events = get_events_with_filter(client, &filter).await?;

	println!("   📊 GAS Contract Events:");
	for event in events {
		println!(
			"      • {} in block {} (TX: {})",
			event.event_name, event.block_height, event.transaction_hash
		);
	}

	Ok(())
}

async fn multi_event_subscription(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔄 Subscribing to multiple event types...");

	// Monitor multiple event types from multiple contracts
	let gas_hash =
		neo3::neo_types::ScriptHash::from_str("d2a4cff31913016155e38e474a2c06d08be276cf")?;
	let neo_hash =
		neo3::neo_types::ScriptHash::from_str("ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5")?;

	let filter = EventFilter {
		contract_hashes: vec![gas_hash, neo_hash],
		event_names: vec![
			"Transfer".to_string(),
			"Approval".to_string(),
			"Mint".to_string(),
			"Burn".to_string(),
		],
		from_block: client.get_block_count().await?.saturating_sub(8),
	};

	let events = get_events_with_filter(client, &filter).await?;

	// Group events by type
	let mut event_counts: HashMap<String, u32> = HashMap::new();
	for event in &events {
		*event_counts.entry(event.event_name.clone()).or_insert(0) += 1;
	}

	println!("   📈 Event Type Distribution:");
	for (event_type, count) in event_counts {
		println!("      • {event_type}: {count} events");
	}

	Ok(())
}

async fn demonstrate_custom_filtering(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔍 Demonstrating custom event filtering...");

	let current_height = client.get_block_count().await?;
	let start_height = current_height.saturating_sub(15);

	println!("   🎯 Custom filters in action:");

	// Filter 1: Only Transfer events with large amounts (conceptual)
	println!("\n   Filter 1: Large Transfer Events");
	let mut large_transfers = 0;

	// Filter 2: Events from contracts deployed recently
	println!("   Filter 2: Events from Recently Deployed Contracts");
	let mut recent_contract_events = 0;

	// Filter 3: Events matching specific patterns
	println!("   Filter 3: Pattern-Based Event Filtering");
	let mut pattern_matches = 0;

	for height in start_height..current_height {
		if let Ok(block) = client.get_block_by_index(height, true).await {
			if let Some(transactions) = &block.transactions {
				for tx in transactions {
					if let Ok(app_log) = client.get_application_log(tx.hash).await {
						for execution in &app_log.executions {
							for notification in &execution.notifications {
								// Custom filter logic
								match notification.event_name.as_str() {
									"Transfer" => {
										// Example: filter by amount (this is conceptual)
										large_transfers += 1;
									},
									_ => {
										// Count events from any contract
										recent_contract_events += 1;
									},
								}

								// Pattern matching example
								if notification.event_name.contains("Transfer")
									|| notification.event_name.contains("Mint")
								{
									pattern_matches += 1;
								}
							}
						}
					}
				}
			}
		}
	}

	println!("      Large transfers found: {large_transfers}");
	println!("      Recent contract events: {recent_contract_events}");
	println!("      Pattern matches: {pattern_matches}");

	println!("\n   💡 Custom filtering capabilities:");
	println!("      • Filter by event parameters/values");
	println!("      • Time-based filtering");
	println!("      • Contract age filtering");
	println!("      • Complex boolean logic");
	println!("      • Pattern matching on event names");

	Ok(())
}

async fn get_events_with_filter(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
	filter: &EventFilter,
) -> Result<Vec<ContractEvent>, Box<dyn std::error::Error>> {
	let mut events = Vec::new();
	let current_height = client.get_block_count().await?;

	for height in filter.from_block..current_height {
		if let Ok(block) = client.get_block_by_index(height, true).await {
			if let Some(transactions) = &block.transactions {
				for tx in transactions {
					if let Ok(app_log) = client.get_application_log(tx.hash).await {
						for execution in &app_log.executions {
							for notification in &execution.notifications {
								// Check contract filter
								let contract_match = filter.contract_hashes.is_empty()
									|| filter.contract_hashes.contains(&notification.contract);

								// Check event name filter
								let event_match = filter.event_names.is_empty()
									|| filter.event_names.contains(&notification.event_name);

								if contract_match && event_match {
									events.push(ContractEvent {
										block_height: height,
										transaction_hash: tx.hash.to_string(),
										contract_hash: notification.contract,
										event_name: notification.event_name.clone(),
										state: Some(notification.state.clone()),
									});
								}
							}
						}
					}
				}
			}
		}
	}

	Ok(events)
}
