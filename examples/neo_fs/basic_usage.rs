use neo3::neo_fs::{
	client::{
		NeoFSAuth, NeoFSClient, NeoFSConfig, DEFAULT_TESTNET_HTTP_GATEWAY, DEFAULT_TESTNET_REST_API,
	},
	container::Container,
	object::Object,
	types::{ContainerId, OwnerId},
	NeoFSService, ObjectType,
};
use reqwest::Client;
use std::env;
use uuid::Uuid;

/// Minimal, runnable NeoFS example that builds real request payloads and probes the public TestNet
/// gateway. If `NEOFS_WALLET` is set, it will also attempt a best-effort container listing.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("📦 Neo N3 NeoFS Basic Usage (TestNet)");
	println!("======================================");

	let endpoint =
		env::var("NEOFS_ENDPOINT").unwrap_or_else(|_| DEFAULT_TESTNET_REST_API.to_string());
	let wallet = env::var("NEOFS_WALLET").ok();

	let auth = wallet
		.as_ref()
		.map(|addr| NeoFSAuth { wallet_address: addr.clone(), private_key: None });
	let config = NeoFSConfig { endpoint: endpoint.clone(), auth, timeout_sec: 10, insecure: false };
	let client = NeoFSClient::new(config.clone());

	println!("🌐 Endpoint: {}", endpoint);
	println!("🔐 Auth: {}", wallet.as_deref().unwrap_or("not provided (read-only probe)"));

	// Build a demo container payload
	let owner_id = OwnerId(wallet.clone().unwrap_or_else(|| "owner-demo-address".to_string()));
	let mut container = Container::new(ContainerId(Uuid::new_v4().to_string()), owner_id.clone())
		.with_basic_acl(0x0F00_0000) // owner full control
		.with_name("demo-container".to_string())
		.with_attribute("purpose", "demo")
		.with_attribute("env", "testnet");
	container.placement_policy.replicas = 1;

	let object = Object::new(container.id.clone().unwrap(), owner_id.clone())
		.with_type(ObjectType::Regular)
		.with_payload(vec![0_u8; 32])
		.with_attribute("content-type", "application/octet-stream")
		.with_attribute("FileName", "demo.bin");

	println!("\n🧱 Container request payload:");
	println!("{}", serde_json::to_string_pretty(&container)?);

	println!("\n📄 Object request payload:");
	println!("{}", serde_json::to_string_pretty(&object)?);

	// Perform a lightweight probe (no panic on failure)
	println!("\n🔎 Probing NeoFS REST gateway...");
	let probe_url = format!("{}/status", DEFAULT_TESTNET_HTTP_GATEWAY.trim_end_matches('/'));
	match Client::new().get(&probe_url).send().await {
		Ok(res) => println!("   ✅ Gateway responded with HTTP {}", res.status()),
		Err(err) => println!("   ⚠️ Gateway probe failed: {err}"),
	}

	// If a wallet address is provided, attempt to list containers (may require valid auth token)
	if wallet.is_some() {
		println!("\n📋 Listing containers (requires valid auth/session)...");
		match client.list_containers().await {
			Ok(ids) => {
				if ids.is_empty() {
					println!("   ℹ️ No containers returned for this account.");
				} else {
					for id in ids {
						println!("   • {}", id.0);
					}
				}
			},
			Err(e) => {
				println!("   ⚠️ list_containers failed (expected without session token): {e}")
			},
		}
	} else {
		println!("\nℹ️ Set NEOFS_WALLET to attempt authenticated container listing.");
	}

	println!("\n✅ NeoFS basic usage demo completed.");
	Ok(())
}
