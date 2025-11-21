use eyre::Result;
use neo3::neo_clients::{APITrait, HttpProvider, RpcClient};
use primitive_types::H256;
use std::env;

/// Fetch a transaction and its execution log from TestNet.
///
/// Run with:
/// `TX_HASH=<txid> cargo run --example trace_transaction`
#[tokio::main]
async fn main() -> Result<()> {
	let tx_hash = env::var("TX_HASH").unwrap_or_else(|_| {
		// A historical TestNet transaction hash; replace with your own for live tracing.
		"0x2762f11163e3e6b166c1f58a1f117e7305b6e3c966d274adb82b0a38d2dc0b36".to_string()
	});

	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);

	let hash = H256::from_slice(&hex::decode(tx_hash.trim_start_matches("0x"))?);

	println!("🔍 Fetching transaction {hash:#x}...");
	let tx = client.get_transaction(hash).await?;
	println!("   • Sender: {:?}", tx.sender);
	println!("   • System fee: {}", tx.sys_fee);
	println!("   • Network fee: {}", tx.net_fee);
	println!("   • Valid until block: {}", tx.valid_until_block);
	println!("   • Attributes: {}", tx.attributes.len());

	println!("📄 Fetching application log...");
	match client.get_application_log(hash).await {
		Ok(log) => {
			println!("   • VM state: {}", log.executions[0].vm_state);
			println!("   • Gas consumed: {}", log.executions[0].gas_consumed);
			println!("   • Notifications: {}", log.executions[0].notifications.len());
			if let Some(stack) = log.executions[0].stack.first() {
				println!("   • First stack item: {:?}", stack);
			}
		},
		Err(e) => {
			println!("   ⚠️  Application log unavailable: {e}");
			println!("   (The transaction may not have executed a contract.)");
		},
	}

	Ok(())
}
