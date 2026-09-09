//! Script builder & transaction assembly benchmarks for the Neo Rust SDK.
//!
//! Micro-benchmarks measure isolated script-builder primitives; macro-benchmarks
//! measure the full transaction-assembly workflow (script + signers + fees +
//! validity window) that an application performs before broadcasting.
//!
//! Run with:
//! ```bash
//! cargo criterion --bench script_builder_benchmarks
//! cargo bench     --bench script_builder_benchmarks
//! ```
//!
//! ## Performance Baselines (v3.3.0, x86_64, release profile)
//!
//! | Operation                          | Class | Target p50 | Target p99 |
//! |------------------------------------|-------|-----------:|-----------:|
//! | `script_builder_simple`            | micro |   ~250 ns  |   <1 µs    |
//! | `script_builder_contract_call`     | micro |   ~1.5 µs  |   <6 µs    |
//! | `transaction_assembly` (macro)     | macro |   ~60 µs   |   <250 µs  |
//!
//! Authoritative numbers live in `target/criterion/**/estimates.json`.

use std::str::FromStr;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use neo3::{
	neo_builder::{AccountSigner, ScriptBuilder, Signer, TransactionBuilder},
	neo_clients::HttpProvider,
	neo_protocol::{Account, AccountTrait},
	neo_types::{ContractParameter, OpCode, ScriptHash},
};
use num_bigint::BigInt;

fn configured_group<'a>(
	c: &'a mut Criterion,
	name: &str,
	sample_size: usize,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
	let mut group = c.benchmark_group(name);
	group
		.warm_up_time(Duration::from_secs(2))
		.measurement_time(Duration::from_secs(5))
		.sample_size(sample_size);
	group
}

// ---------------------------------------------------------------------------
// MICRO-BENCHMARKS: script builder primitives
// ---------------------------------------------------------------------------

fn bench_script_primitives(c: &mut Criterion) {
	let mut group = configured_group(c, "script/primitives", 200);

	group.bench_function("script_builder_simple", |b| {
		b.iter(|| {
			let mut builder = ScriptBuilder::new();
			builder.push_integer(std::hint::black_box(BigInt::from(42)));
			builder.push_data(std::hint::black_box(b"test".to_vec()));
			builder.op_code(&[std::hint::black_box(OpCode::Add)]);
			std::hint::black_box(builder.to_bytes())
		});
	});

	let contract_hash =
		ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf").unwrap();
	group.bench_function("script_builder_contract_call", |b| {
		b.iter(|| {
			let mut builder = ScriptBuilder::new();
			builder
				.contract_call(
					std::hint::black_box(&contract_hash),
					std::hint::black_box("transfer"),
					std::hint::black_box(&[
						ContractParameter::h160(&contract_hash),
						ContractParameter::h160(&contract_hash),
						ContractParameter::integer(100),
						ContractParameter::any(),
					]),
					None,
				)
				.unwrap();
			std::hint::black_box(builder.to_bytes())
		});
	});

	group.finish();
}

// ---------------------------------------------------------------------------
// MACRO-BENCHMARK: full transaction assembly workflow
// ---------------------------------------------------------------------------

fn bench_transaction_assembly(c: &mut Criterion) {
	let account = Account::create().unwrap();

	let mut group = configured_group(c, "script/transaction", 150);
	group.bench_function("transaction_assembly", |b| {
		// Benchmark the assembly workflow and verify it compiles
		b.iter_batched(
			|| (),
			|_| {
				let mut builder: TransactionBuilder<HttpProvider> = TransactionBuilder::new();
				builder.set_script(Some(vec![0x00, 0x01, 0x02]));
				let signer: Signer = AccountSigner::called_by_entry_hash160(account.get_script_hash())
					.unwrap()
					.into();
				let _ = builder.set_signers(vec![signer]);
				builder.nonce(12345).unwrap();
				builder.set_additional_system_fee(1_000_000);
				builder.set_additional_network_fee(500_000);
				builder.valid_until_block(1_000_000).unwrap();
				// Move the tx out so black_box doesn't keep references alive.
				// Use a synchronous executor for criterion's batch mode
				let rt = tokio::runtime::Runtime::new().unwrap();
				let tx = rt.block_on(async { builder.build().await })
					.expect("build failed");
				std::hint::black_box(&tx);
			},
			criterion::BatchSize::SmallInput,
		)
	});
	group.finish();
}

criterion_group!(
	script_builder_benches,
	bench_script_primitives,
	bench_transaction_assembly,
);
criterion_main!(script_builder_benches);
