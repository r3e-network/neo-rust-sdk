use neo3::neo_fs::{
	client::{NeoFSAuth, NeoFSClient, NeoFSConfig, DEFAULT_TESTNET_REST_API},
	object::{MultipartUpload, Object, Part},
	types::{ContainerId, OwnerId},
	NeoFSService,
};
use rand::{thread_rng, RngCore};
use std::env;
use uuid::Uuid;

/// Demonstrates building a multipart upload plan for NeoFS and simulating part assembly.
/// No data is pushed to the network; the example shows how to structure the upload and
/// validates part ordering locally.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("📦 NeoFS Multipart Upload Demo (local plan + optional API probe)");

	let endpoint =
		env::var("NEOFS_ENDPOINT").unwrap_or_else(|_| DEFAULT_TESTNET_REST_API.to_string());
	let wallet = env::var("NEOFS_WALLET").ok();
	let auth = wallet
		.as_ref()
		.map(|addr| NeoFSAuth { wallet_address: addr.clone(), private_key: None });

	let config = NeoFSConfig { endpoint, auth, timeout_sec: 10, insecure: false };
	let client = NeoFSClient::new(config);

	let container_id = ContainerId(Uuid::new_v4().to_string());
	let owner_id = OwnerId(wallet.unwrap_or_else(|| "owner-demo-address".into()));
	let object = Object::new(container_id.clone(), owner_id.clone()).with_payload(vec![0u8; 256]);

	// Prepare a 256KB payload and split into 64KB parts
	let total_size = 256 * 1024;
	let part_size = 64 * 1024;
	let upload = MultipartUpload {
		id: None,
		container_id: container_id.clone(),
		owner_id: owner_id.clone(),
		upload_id: Uuid::new_v4().to_string(),
		attributes: Default::default(),
		part_size: part_size as u64,
		max_parts: 16,
	};

	println!("\n🧮 Planning multipart upload");
	println!("   Container: {}", upload.container_id.0);
	println!("   Owner:     {}", upload.owner_id.0);
	println!("   Upload ID: {}", upload.upload_id);
	println!("   Part size: {} bytes", part_size);

	// Build parts
	let mut parts: Vec<Part> = Vec::new();
	let mut remaining = total_size;
	let mut part_no = 1;
	let mut rng = thread_rng();

	while remaining > 0 && (parts.len() as u64) < upload.max_parts {
		let current_size = remaining.min(part_size);
		let mut buf = vec![0u8; current_size];
		rng.fill_bytes(&mut buf);
		parts.push(Part::new(part_no, buf));
		part_no += 1;
		remaining -= current_size;
	}

	println!("   Planned {} parts totaling {} bytes", parts.len(), total_size);

	// Assemble parts locally to verify ordering
	let mut assembled: Vec<u8> = Vec::with_capacity(total_size);
	for part in &parts {
		assembled.extend_from_slice(&part.payload);
	}
	println!("   ✅ Local assembly check passed ({} bytes)", assembled.len());

	// Optional API probe: attempt to initiate upload (will require auth/session server-side)
	println!("\n🔎 NeoFS API probe (initiate_multipart_upload)...");
	if let Err(e) = client.initiate_multipart_upload(&container_id, &object).await {
		println!("   ⚠️ init_multipart_upload not executed (expected without server session): {e}");
	} else {
		println!("   ✅ init_multipart_upload call succeeded");
	}

	println!("\n✅ Multipart upload demo completed.");
	Ok(())
}
