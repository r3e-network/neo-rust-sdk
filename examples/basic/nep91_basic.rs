/*!
# NEP-91 Account Events - Basic Usage Examples

This example demonstrates how to query account events using the NEP-91
`AccountEventQuery` builder. It covers:

- Basic event queries scanning a block range (`execute`)
- Fast-path optimization for `Transfer` events (`execute_fast`)
- Pagination through large result sets (`offset` + `page_size`)
- Filtering by contract and event name

## Notes:
- The `execute` path scans every transaction in every block of the range and
  can be very expensive over wide ranges — prefer `execute_fast` for
  `Transfer`-only queries.
- All queries below run against public TestNet endpoints and print results;
  they do not require a funded wallet.
*/

use neo3::{
	neo_clients::{APITrait, HttpProvider, RpcClient},
	neo_contract::AccountEventQuery,
	neo_types::ScriptHash,
};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
	let client = RpcClient::new(provider);

	// The GAS token is a NEP-17 contract, so its `Transfer` events are indexed
	// and can be queried through the fast path.
	let gas_token = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf")?;

	// A sample account address (Base58Check `N...`). Replace with your own.
	let account_address = "NgPkjjLTNcQad99iRYeXRUuowE4gxLAnDL";

	// Confirm the node is reachable before issuing queries.
	match client.get_block_count().await {
		Ok(height) => println!("🟢 Connected to TestNet (height: {height})\n"),
		Err(e) => {
			eprintln!("❌ Cannot reach node: {e}");
			return Ok(());
		},
	}

	basic_transfer_scan(&client, account_address).await;
	fast_path_transfers(&client, account_address, gas_token).await;
	paginated_scan(&client, account_address).await;
	filter_by_contract_and_event(&client, account_address, gas_token).await;

	println!("\n💡 Tip: for `Transfer`-only queries prefer `execute_fast`; it uses");
	println!("   the indexed `getnep17transfers` RPC instead of scanning blocks.");
	Ok(())
}

/// Pattern 1: scan a block range for every `Transfer` referencing the account.
async fn basic_transfer_scan(client: &RpcClient<HttpProvider>, account_address: &str) {
	println!("📋 Pattern 1 — block-range scan (execute):");

	// `address(..)` decodes a Base58Check `N...` address into a script hash.
	let query = match AccountEventQuery::address(account_address) {
		Ok(q) => q.event_name("Transfer").block_range(0, 50).limit(10),
		Err(e) => {
			eprintln!("   invalid address: {e}");
			return;
		},
	};

	match query.execute(client).await {
		Ok(events) => {
			println!("   found {} event(s)", events.len());
			for e in events.iter().take(3) {
				println!("   • block {} tx {:?}", e.block_index, e.tx_hash);
			}
		},
		Err(e) => println!("   error: {e}"),
	}
}

/// Pattern 2: use the fast path for `Transfer` events on a known NEP-17 token.
async fn fast_path_transfers(
	client: &RpcClient<HttpProvider>,
	account_address: &str,
	token: ScriptHash,
) {
	println!("\n⚡ Pattern 2 — fast path (execute_fast via getnep17transfers):");

	let query = match AccountEventQuery::address(account_address) {
		Ok(q) => q.contract(token).event_name("Transfer").block_range(0, 5_000_000).limit(10),
		Err(e) => {
			eprintln!("   invalid address: {e}");
			return;
		},
	};

	match query.execute_fast(client).await {
		Ok(events) => {
			println!("   found {} transfer(s) with one indexed RPC call", events.len());
			for e in events.iter().take(3) {
				println!("   • block {} token {:?}", e.block_index, e.contract);
			}
		},
		Err(e) => println!("   error: {e}"),
	}
}

/// Pattern 3: page through a wide range with `offset` + `page_size`.
async fn paginated_scan(client: &RpcClient<HttpProvider>, account_address: &str) {
	println!("\n📄 Pattern 3 — pagination (offset + page_size):");

	let page_size = 25usize;
	for page in 0..3usize {
		let offset = page * page_size;
		let query = match AccountEventQuery::address(account_address) {
			Ok(q) => q
				.event_name("Transfer")
				.block_range(0, 100)
				.offset(offset)
				.page_size(page_size)
				.limit(page_size),
			Err(e) => {
				eprintln!("   invalid address: {e}");
				return;
			},
		};

		match query.execute(client).await {
			Ok(events) if !events.is_empty() => {
				println!("   page {} (offset {}): {} result(s)", page + 1, offset, events.len());
			},
			Ok(_) => {
				println!("   page {} (offset {}): no more results", page + 1, offset);
				break;
			},
			Err(e) => {
				println!("   page {} error: {e}", page + 1);
				break;
			},
		}
	}
}

/// Pattern 4: filter by both emitting contract and event name.
async fn filter_by_contract_and_event(
	client: &RpcClient<HttpProvider>,
	account_address: &str,
	token: ScriptHash,
) {
	println!("\n🔍 Pattern 4 — dual filter (contract + event name):");

	let query = match AccountEventQuery::address(account_address) {
		Ok(q) => q.contract(token).event_name("Transfer").block_range(0, 100).limit(20),
		Err(e) => {
			eprintln!("   invalid address: {e}");
			return;
		},
	};

	match query.execute(client).await {
		Ok(events) => println!("   found {} matching event(s)", events.len()),
		Err(e) => println!("   error: {e}"),
	}
}
