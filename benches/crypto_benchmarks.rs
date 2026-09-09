//! Cryptographic operation benchmarks for the Neo Rust SDK.
//!
//! These benchmarks establish performance baselines for the security-critical
//! cryptographic primitives used throughout the SDK: ECDSA key generation,
//! signing, signature verification, hashing, and address/WIF derivation.
//!
//! Run with:
//! ```bash
//! cargo criterion --bench crypto_benchmarks          # statistical, HTML reports
//! cargo bench     --bench crypto_benchmarks          # plain cargo fallback
//! ```
//!
//! ## Performance Baselines (v3.3.0, x86_64, release profile)
//!
//! | Operation                     | Class | Target p50 | Target p99 |
//! |-------------------------------|-------|-----------:|-----------:|
//! | `key_pair_generation`         | micro |   ~50 µs   |   <150 µs  |
//! | `ecdsa_sign_prehash`          | micro |   ~35 µs   |   <10 ms   | <- Phase 5 KPI
//! | `ecdsa_verify`                | micro |   ~70 µs   |   <200 µs  |
//! | `hash256_1kb`                 | micro |   ~2 µs    |   <5 µs    |
//! | `hash256_1mb`                 | micro |   ~1.8 ms  |   <4 ms    |
//! | `account_create` (macro)      | macro |   ~55 µs   |   <200 µs  |
//! | `wif_roundtrip` (macro)       | macro |   ~60 µs   |   <200 µs  |
//!
//! Baselines are indicative; the authoritative numbers live in Criterion's
//! `target/criterion/**/estimates.json` and are compared automatically in CI.
//! The Phase 5 acceptance KPI is: p99 latency for single-key signing < 10 ms.

use std::time::Duration;

use criterion::{
	criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use neo3::{
	neo_crypto::{HashableForVec, KeyPair},
	neo_protocol::{Account, AccountTrait},
};

/// Shared measurement configuration so every group has consistent, statistically
/// meaningful warmup and sampling. Micro-benchmarks are extremely fast, so we
/// use a generous sample count to tighten the confidence interval that feeds
/// the p95/p99 percentile estimates.
fn micro_group<'a>(
	c: &'a mut Criterion,
	name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
	let mut group = c.benchmark_group(name);
	group
		.warm_up_time(Duration::from_secs(2))
		.measurement_time(Duration::from_secs(5))
		.sample_size(200)
		.sampling_mode(SamplingMode::Auto);
	group
}

// ---------------------------------------------------------------------------
// MICRO-BENCHMARKS: individual cryptographic primitives
// ---------------------------------------------------------------------------

/// ECDSA key-pair generation over secp256r1.
fn bench_key_generation(c: &mut Criterion) {
	let mut group = micro_group(c, "crypto/keygen");
	group.bench_function("key_pair_generation", |b| {
		b.iter(|| std::hint::black_box(KeyPair::new_random()))
	});
	group.finish();
}

/// ECDSA signing over a pre-hashed 32-byte digest. This is the Phase 5 KPI
/// path: p99 must remain below 10 ms.
fn bench_signing(c: &mut Criterion) {
	let key_pair = KeyPair::new_random();
	let message = b"Neo N3 signing benchmark payload - deterministic content.";
	let message_hash = message.hash256();

	let mut group = micro_group(c, "crypto/sign");
	group.bench_function("ecdsa_sign_prehash", |b| {
		b.iter(|| {
			std::hint::black_box(
				key_pair.private_key_ref().unwrap().sign_prehash(&message_hash).unwrap(),
			)
		});
	});
	group.finish();
}

/// ECDSA signature verification.
fn bench_verification(c: &mut Criterion) {
	let key_pair = KeyPair::new_random();
	let message = b"Neo N3 signing benchmark payload - deterministic content.";
	let message_hash = message.hash256();
	let signature = key_pair.private_key_ref().unwrap().sign_prehash(&message_hash).unwrap();

	let mut group = micro_group(c, "crypto/verify");
	group.bench_function("ecdsa_verify", |b| {
		b.iter(|| {
			std::hint::black_box(
				key_pair.public_key_ref().verify(&message_hash, &signature).is_ok(),
			)
		});
	});
	group.finish();
}

/// SHA-256 double-hash (`hash256`) across representative payload sizes.
/// Throughput is reported so byte-normalised regressions are visible.
fn bench_hashing(c: &mut Criterion) {
	let mut group = micro_group(c, "crypto/hash256");
	// Hashing large buffers is slower; reduce sample size to keep wall-time sane.
	group.sample_size(100);

	for size in [64usize, 256, 1024, 16 * 1024, 1024 * 1024] {
		let data = vec![0u8; size];
		group.throughput(Throughput::Bytes(size as u64));
		group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
			b.iter(|| std::hint::black_box(data.hash256()))
		});
	}
	group.finish();
}

// ---------------------------------------------------------------------------
// MACRO-BENCHMARKS: full derivation workflows built on the primitives above
// ---------------------------------------------------------------------------

/// Full account creation workflow (keygen + address derivation + metadata).
fn bench_account_creation(c: &mut Criterion) {
	let mut group = micro_group(c, "crypto/account");
	group.bench_function("account_create", |b| {
		b.iter(|| std::hint::black_box(Account::create().unwrap()))
	});

	let key_pair = KeyPair::new_random();
	group.bench_function("address_from_public_key", |b| {
		b.iter(|| {
			let account = Account::from_key_pair(key_pair.clone(), None, None).unwrap();
			std::hint::black_box(account.get_address())
		});
	});
	group.finish();
}

/// WIF export/import round-trip (base58 + checksum + key material handling).
fn bench_wif_roundtrip(c: &mut Criterion) {
	let key_pair = KeyPair::new_random();
	let wif = key_pair.export_as_wif().unwrap();

	let mut group = micro_group(c, "crypto/wif");
	group.bench_function("wif_export", |b| {
		b.iter(|| std::hint::black_box(key_pair.export_as_wif().unwrap()))
	});
	group.bench_function("wif_import", |b| {
		b.iter(|| std::hint::black_box(Account::from_wif(&wif).unwrap()))
	});
	group.bench_function("wif_roundtrip", |b| {
		b.iter(|| {
			let exported = key_pair.export_as_wif().unwrap();
			std::hint::black_box(Account::from_wif(&exported).unwrap())
		})
	});
	group.finish();
}

criterion_group!(
	crypto_benches,
	bench_key_generation,
	bench_signing,
	bench_verification,
	bench_hashing,
	bench_account_creation,
	bench_wif_roundtrip,
);
criterion_main!(crypto_benches);
