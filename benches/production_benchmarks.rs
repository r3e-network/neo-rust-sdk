//! Production macro-benchmark suite for the Neo Rust SDK (v3.3.0 Phase 5).
//!
//! Where the per-domain benches (`crypto_`, `gas_`, `script_`, `wallet_`)
//! measure isolated primitives, this suite measures **end-to-end workflows**
//! and **throughput under load** — the numbers that matter for production
//! capacity planning and month-over-month regression tracking.
//!
//! Suites (mirrors the v3.3.0 plan: "cryptographic operations, serialization,
//! RPC batching"):
//!   1. `prod/sign_pipeline`   — keygen → hash → sign → verify round-trip
//!   2. `prod/serialization`   — script assembly + hashing throughput (bytes/s)
//!   3. `prod/batch_signing`   — N-message batch signing (RPC-batch analogue)
//!
//! Run with:
//! ```bash
//! cargo criterion --bench production_benchmarks
//! cargo bench     --bench production_benchmarks
//! ```
//!
//! ## p95 / p99 latency tracking
//!
//! Criterion records the full sample distribution; percentile estimates are
//! written to `target/criterion/<group>/<bench>/new/estimates.json`. The CI
//! workflow (`.github/workflows/benchmark.yml`) diffs the `mean`/`slope`
//! against the committed baseline and fails on a >5% regression. The Phase 5
//! acceptance KPI is: **p99 single-key signing < 10 ms**.
//!
//! ## Baselines (v3.3.0, x86_64, release profile — indicative)
//!
//! | Benchmark                         | Target p50 | Target p99 |
//! |-----------------------------------|-----------:|-----------:|
//! | `sign_pipeline/full_roundtrip`    |   ~110 µs  |   <400 µs  |
//! | `serialization/script_1kb`        |   ~8 µs    |   <30 µs   |
//! | `batch_signing/64`                |   ~2.3 ms  |   <8 ms    |

use std::time::Duration;

use criterion::{
	criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use neo3::{
	neo_builder::ScriptBuilder,
	neo_crypto::{HashableForVec, KeyPair},
	neo_types::OpCode,
};
use num_bigint::BigInt;

fn prod_group<'a>(
	c: &'a mut Criterion,
	name: &str,
	sample_size: usize,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
	let mut group = c.benchmark_group(name);
	group
		.warm_up_time(Duration::from_secs(2))
		.measurement_time(Duration::from_secs(6))
		.sample_size(sample_size)
		.sampling_mode(SamplingMode::Auto);
	group
}

// ---------------------------------------------------------------------------
// Suite 1: full sign/verify pipeline (cryptographic operations)
// ---------------------------------------------------------------------------

/// End-to-end pipeline a signer performs per transaction: hash the payload,
/// produce an ECDSA signature, then verify it. Represents the hot path used by
/// wallet/transaction broadcasting.
fn bench_sign_pipeline(c: &mut Criterion) {
	let mut group = prod_group(c, "prod/sign_pipeline", 200);

	let key_pair = KeyPair::new_random();
	let payload = b"Neo N3 production sign pipeline payload for macro benchmarking.";

	group.bench_function("full_roundtrip", |b| {
		b.iter(|| {
			let hash = std::hint::black_box(payload).hash256();
			let sig = key_pair.private_key_ref().unwrap().sign_prehash(&hash).unwrap();
			let ok = key_pair.public_key_ref().verify(&hash, &sig).is_ok();
			std::hint::black_box(ok)
		})
	});

	group.finish();
}

// ---------------------------------------------------------------------------
// Suite 2: serialization throughput (bytes/s)
// ---------------------------------------------------------------------------

/// Assemble scripts of increasing size and hash the resulting bytes. Throughput
/// is reported in bytes so regressions in serialization/hashing efficiency are
/// visible independent of payload size.
fn bench_serialization(c: &mut Criterion) {
	let mut group = prod_group(c, "prod/serialization", 100);

	for elements in [64u64, 256, 1024, 4096] {
		// Build once to measure the representative script size for throughput.
		let script = build_script(elements);
		group.throughput(Throughput::Bytes(script.len() as u64));
		group.bench_with_input(
			BenchmarkId::new("script_build_and_hash", elements),
			&elements,
			|b, &elements| {
				b.iter(|| {
					let bytes = build_script(std::hint::black_box(elements));
					std::hint::black_box(bytes.hash256())
				})
			},
		);
	}

	group.finish();
}

fn build_script(elements: u64) -> Vec<u8> {
	let mut builder = ScriptBuilder::new();
	for i in 0..elements {
		builder.push_integer(BigInt::from(i));
	}
	builder.op_code(&[OpCode::Nop]).to_bytes()
}

// ---------------------------------------------------------------------------
// Suite 3: batch signing (RPC-batching analogue, throughput under load)
// ---------------------------------------------------------------------------

/// Sign a batch of N distinct message digests with a single key. Models the
/// load profile of batched transaction submission where many payloads are
/// signed back-to-back. Throughput is reported in signatures/s (Elements).
fn bench_batch_signing(c: &mut Criterion) {
	let mut group = prod_group(c, "prod/batch_signing", 50);

	let key_pair = KeyPair::new_random();

	for batch in [8u64, 32, 64, 256] {
		// Pre-compute the digests so we measure signing, not hashing.
		let digests: Vec<Vec<u8>> = (0..batch)
			.map(|i| format!("neo-batch-message-{i}").into_bytes().hash256())
			.collect();

		group.throughput(Throughput::Elements(batch));
		group.bench_with_input(BenchmarkId::from_parameter(batch), &digests, |b, digests| {
			b.iter(|| {
				let priv_key = key_pair.private_key_ref().unwrap();
				for digest in digests {
					std::hint::black_box(priv_key.sign_prehash(digest).unwrap());
				}
			})
		});
	}

	group.finish();
}

criterion_group!(
	production_benches,
	bench_sign_pipeline,
	bench_serialization,
	bench_batch_signing,
);
criterion_main!(production_benches);
