/// Neo N3 Event Monitoring Example
///
/// This example demonstrates how to monitor contract events and notifications
/// in Neo N3 by polling blocks and parsing application logs.
use neo3::neo_clients::APITrait;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🎯 Neo N3 Event Monitoring Example");
	println!("==================================\n");

	// Connect to TestNet
	let client = connect_to_testnet().await?;

	// 1. Monitor recent blocks for events
	println!("1️⃣ Monitoring Recent Block Events...");
	monitor_recent_events(&client).await?;

	// 2. Look for specific contract events
	println!("\n2️⃣ Filtering Contract-Specific Events...");
	monitor_contract_events(&client).await?;

	// 3. Parse different event types
	println!("\n3️⃣ Parsing Different Event Types...");
	parse_event_types(&client).await?;

	// 4. Demonstrate continuous monitoring
	println!("\n4️⃣ Continuous Event Monitoring (Demo)...");
	demonstrate_continuous_monitoring(&client).await?;

	println!("\n✅ Event monitoring example completed!");
	println!("💡 This shows how to implement real event monitoring in Neo N3");

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

async fn monitor_recent_events(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	let current_height = client.get_block_count().await?;
	let start_height = current_height.saturating_sub(5); // Check last 5 blocks

	println!("   📊 Scanning blocks {} to {}", start_height, current_height - 1);

	let mut total_transactions = 0;
	let mut events_found = 0;

	for height in start_height..current_height {
		match client.get_block_by_index(height, true).await {
			Ok(block) => {
				if let Some(transactions) = &block.transactions {
					total_transactions += transactions.len();

					for tx in transactions {
						// Get application log for each transaction
						match client.get_application_log(tx.hash).await {
							Ok(app_log) => {
								if !app_log.executions.is_empty() {
									for execution in &app_log.executions {
										if !execution.notifications.is_empty() {
											events_found += execution.notifications.len();

											for notification in &execution.notifications {
												println!("      📢 Event found in block {height}:");
												println!(
													"         Contract: 0x{:x}",
													notification.contract
												);
												println!(
													"         Event: {}",
													notification.event_name
												);
												println!("         TX: {}", tx.hash);
											}
										}
									}
								}
							},
							Err(_) => continue, // Some transactions might not have logs
						}
					}
				}
			},
			Err(e) => println!("      ❌ Failed to get block {height}: {e}"),
		}
	}

	println!("   📈 Summary:");
	println!("      Blocks scanned: {}", current_height - start_height);
	println!("      Transactions: {total_transactions}");
	println!("      Events found: {events_found}");

	Ok(())
}

async fn monitor_contract_events(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	// Monitor specific contracts
	let contracts_to_monitor = vec![
		("GAS", "d2a4cff31913016155e38e474a2c06d08be276cf"),
		("NEO", "ef4073a0f2b305a38ec4050e4d3d28bc40ea63f5"),
	];

	println!("   🎯 Monitoring specific contracts:");

	let current_height = client.get_block_count().await?;
	let start_height = current_height.saturating_sub(10); // Check last 10 blocks

	let mut contract_events: HashMap<String, u32> = HashMap::new();

	for height in start_height..current_height {
		if let Ok(block) = client.get_block_by_index(height, true).await {
			if let Some(transactions) = &block.transactions {
				for tx in transactions {
					if let Ok(app_log) = client.get_application_log(tx.hash).await {
						for execution in &app_log.executions {
							for notification in &execution.notifications {
								let contract_hex = format!("{:x}", notification.contract);

								// Check if this is a contract we're monitoring
								for (name, hash) in &contracts_to_monitor {
									if contract_hex == *hash {
										let key = format!("{name}::{}", notification.event_name);
										*contract_events.entry(key).or_insert(0) += 1;

										println!(
											"      📢 {name} event: {}",
											notification.event_name
										);
										println!("         Block: {height}, TX: {}", tx.hash);
									}
								}
							}
						}
					}
				}
			}
		}
	}

	println!("\n   📊 Contract event summary:");
	for (event, count) in contract_events {
		println!("      • {event}: {count} events");
	}

	Ok(())
}

async fn parse_event_types(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔍 Parsing different event types:");

	let current_height = client.get_block_count().await?;
	let start_height = current_height.saturating_sub(3);

	for height in start_height..current_height {
		if let Ok(block) = client.get_block_by_index(height, true).await {
			if let Some(transactions) = &block.transactions {
				for tx in transactions {
					if let Ok(app_log) = client.get_application_log(tx.hash).await {
						for execution in &app_log.executions {
							for notification in &execution.notifications {
								match notification.event_name.as_str() {
									"Transfer" => {
										println!("      💸 Transfer Event:");
										println!(
											"         Contract: 0x{:x}",
											notification.contract
										);
										{
											let state = &notification.state;
											println!("         State: {state:?}");
										}
									},
									"Mint" => {
										println!("      🪙 Mint Event:");
										println!(
											"         Contract: 0x{:x}",
											notification.contract
										);
									},
									"Burn" => {
										println!("      🔥 Burn Event:");
										println!(
											"         Contract: 0x{:x}",
											notification.contract
										);
									},
									other => {
										println!("      🔔 {other} Event:");
										println!(
											"         Contract: 0x{:x}",
											notification.contract
										);
									},
								}
							}
						}
					}
				}
			}
		}
	}

	Ok(())
}

async fn demonstrate_continuous_monitoring(
	client: &neo3::neo_clients::RpcClient<neo3::neo_clients::HttpProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
	println!("   🔄 Demonstrating continuous monitoring (5 second demo):");

	let start_height = client.get_block_count().await?;
	let mut last_checked_height = start_height;

	// Monitor for 5 seconds
	let start_time = std::time::Instant::now();
	while start_time.elapsed() < Duration::from_secs(5) {
		let current_height = client.get_block_count().await?;

		if current_height > last_checked_height {
			println!("      📦 New block #{current_height} detected!");

			// Check the new block for events
			if let Ok(block) = client.get_block_by_index(current_height - 1, true).await {
				if let Some(transactions) = &block.transactions {
					println!("         Transactions in block: {}", transactions.len());

					for tx in transactions {
						if let Ok(app_log) = client.get_application_log(tx.hash).await {
							let total_events: usize = app_log
								.executions
								.iter()
								.map(|exec| exec.notifications.len())
								.sum();

							if total_events > 0 {
								println!("         TX {} has {} events", tx.hash, total_events);
							}
						}
					}
				}
			}

			last_checked_height = current_height;
		}

		sleep(Duration::from_millis(500)).await;
	}

	println!("      ✅ Monitoring demo completed");
	println!("\n   💡 Real implementation would:");
	println!("      • Use WebSocket connections when available");
	println!("      • Implement backoff strategies");
	println!("      • Store processed block heights");
	println!("      • Handle network failures gracefully");
	println!("      • Filter events by specific criteria");

	Ok(())
}