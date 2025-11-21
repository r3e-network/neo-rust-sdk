use neo3::{
	neo_builder::{CallFlags, ScriptBuilder, Signer},
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_crypto::HashableForVec,
	neo_protocol::{Account, AccountTrait},
	neo_types::{ContractParameter, NeoVMStateType, ScriptHash},
};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

/// Demonstrates how to build a deploy transaction using a real NEF + manifest,
/// simulate it, and compute the expected contract hash. This example does not
/// broadcast the transaction; it focuses on creating a working deployment script.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🚀 Neo N3 Smart Contract Deployment Example (TestNet)");
	println!("====================================================");

	// 1) Connect to TestNet
	let client = RpcClient::new(HttpProvider::new("https://testnet1.neo.org:443")?);
	println!("   ✅ Connected to TestNet");

	// 2) Load deployer (demo WIF; replace for production)
	let deployer_wif = "L1eV34wPoj9weqhGijdDLtVQzUpWGHszXXpdU9dPuh2nRFFzFa7E";
	let deployer = Account::from_wif(deployer_wif)?;
	println!("   📍 Deployer: {}", deployer.get_address());

	// 3) Load NEF + manifest fixtures from repo
	let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
	let nef_bytes = std::fs::read(
		manifest_dir
			.join("../..")
			.join("test_resources/responses/contract/contracts/TestContract.nef"),
	)?;
	let manifest_json = std::fs::read_to_string(
		manifest_dir
			.join("../..")
			.join("test_resources/responses/contract/contracts/TestContract.manifest.json"),
	)?;
	println!(
		"   📦 Loaded NEF ({} bytes) and manifest ({} bytes)",
		nef_bytes.len(),
		manifest_json.len()
	);

	// 4) Build deploy script (ContractManagement.deploy)
	let mgmt_hash = ScriptHash::from_str("fffdc93764dbaddd97c48f252a53ea4643faa3fd")?;
	let deploy_params = vec![
		ContractParameter::byte_array(nef_bytes.clone()),
		ContractParameter::string(manifest_json.clone()),
		ContractParameter::any(),
	];
	let deploy_script = ScriptBuilder::new()
		.contract_call(&mgmt_hash, "deploy", &deploy_params, Some(CallFlags::All))
		.unwrap()
		.to_bytes();
	println!("   🔧 Deployment script size: {} bytes", deploy_script.len());

	// 5) Simulate deployment to inspect VM state and gas
	match client.invoke_script(hex::encode(&deploy_script), vec![]).await {
		Ok(sim) => {
			println!("   🧪 Simulation state: {:?}", sim.state);
			println!("   ⛽ Gas consumed (simulation): {}", sim.gas_consumed);
			println!("   🧱 Stack items returned: {}", sim.stack.len());
		},
		Err(e) => {
			println!("   ⚠️  Simulation failed: {e}");
			println!("      (Ensure the node is reachable and accepts invokeScript calls)");
		},
	};

	// 6) Compute expected contract hash (simplified demo hash)
	let expected_hash =
		calculate_contract_hash(&deployer.get_script_hash(), &manifest_json)?;
	println!("   🔑 Expected contract hash: {}", expected_hash.to_string());

	// 7) (Optional) Build a transaction with signer; not signed/broadcast here
	let mut builder: neo3::neo_builder::TransactionBuilder<HttpProvider> =
		neo3::neo_builder::TransactionBuilder::new();
	builder.set_script(Some(deploy_script.clone()));
	builder.set_signers(vec![Signer::AccountSigner(
		neo3::neo_builder::AccountSigner::called_by_entry_hash160(deployer.get_script_hash())?,
	)])?;
	let block_height = match client.get_block_count().await {
		Ok(h) => h,
		Err(e) => {
			println!("   ⚠️  Could not fetch latest block height: {e}");
			0
		},
	};
	builder.valid_until_block(block_height + 1000)?;
	println!("   📝 Transaction ready for signing (valid until block {})", block_height + 1000);
	println!("   💡 Sign and send with your own key when ready.");

	Ok(())
}

fn calculate_contract_hash(
	sender: &neo3::neo_types::ScriptHash,
	manifest_json: &str,
) -> Result<neo3::neo_types::ScriptHash, Box<dyn std::error::Error>> {
	// Demo-only hash: SHA256(sender || manifest_json) then take first 20 bytes.
	let mut data = Vec::new();
	data.extend_from_slice(sender.as_bytes());
	data.extend_from_slice(manifest_json.as_bytes());

	let hash = data.hash256();
	Ok(neo3::neo_types::ScriptHash::from_slice(&hash[..20]))
}

fn _assert_display_state(state: NeoVMStateType) -> String {
	format!("{:?}", state)
}
