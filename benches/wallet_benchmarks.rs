//! Wallet operation benchmarks for the Neo Rust SDK.
//!
//! Establishes baselines for wallet lifecycle operations: creation, account
//! addition/derivation, scrypt-based encryption/decryption, password
//! verification, and backup/recovery round-trips.
//!
//! Run with:
//! ```bash
//! cargo criterion --bench wallet_benchmarks
//! cargo bench     --bench wallet_benchmarks
//! ```
//!
//! ## Performance Baselines (v3.3.0, x86_64, release profile)
//!
//! | Operation                              | Class | Note                       |
//! |----------------------------------------|-------|----------------------------|
//! | `wallet_creation`                      | micro | allocation only            |
//! | `account_addition`                     | micro | keygen dominated           |
//! | `encrypt_1_account`                    | macro | scrypt KDF dominated       |
//! | `encrypt_parallel_10_accounts`         | macro | rayon-parallel scrypt      |
//! | `password_verify_correct`              | macro | single scrypt decrypt      |
//! | `backup_5_accounts` / `recover_*`      | macro | serialization + IO         |
//!
//! NOTE: scrypt is intentionally CPU-expensive, so encryption/decryption
//! benchmarks use small sample sizes to keep wall-clock time bounded while
//! still yielding stable p50/p95/p99 estimates. Absolute timings are dominated
//! by the wallet's configured scrypt work factor, not SDK overhead.

use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use neo3::{
	neo_protocol::{Account, AccountTrait},
	neo_wallets::{Wallet, WalletBackup, WalletTrait},
};
use tempfile::TempDir;

/// Fast group config for cheap operations (creation, account add).
fn fast_group<'a>(
	c: &'a mut Criterion,
	name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
	let mut group = c.benchmark_group(name);
	group
		.warm_up_time(Duration::from_secs(1))
		.measurement_time(Duration::from_secs(4))
		.sample_size(150);
	group
}

/// Slow group config for scrypt-heavy operations. Small sample size keeps the
/// overall run bounded; Criterion still computes robust percentile estimates.
fn kdf_group<'a>(
	c: &'a mut Criterion,
	name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
	let mut group = c.benchmark_group(name);
	group
		.warm_up_time(Duration::from_secs(1))
		.measurement_time(Duration::from_secs(8))
		.sample_size(10);
	group
}

fn seeded_wallet(count: usize) -> Wallet {
	let mut wallet = Wallet::new();
	for _ in 0..count {
		wallet.add_account(Account::create().unwrap());
	}
	wallet
}

// ---------------------------------------------------------------------------
// MICRO-BENCHMARKS: wallet structure & account derivation
// ---------------------------------------------------------------------------

fn bench_wallet_lifecycle(c: &mut Criterion) {
	let mut group = fast_group(c, "wallet/lifecycle");

	group.bench_function("wallet_creation", |b| {
		b.iter(|| std::hint::black_box(Wallet::new()))
	});

	group.bench_function("account_addition", |b| {
		b.iter(|| {
			let mut wallet = Wallet::new();
			let account = Account::create().unwrap();
			wallet.add_account(account);
			std::hint::black_box(())
		});
	});

	group.finish();
}

// ---------------------------------------------------------------------------
// MACRO-BENCHMARKS: encryption / decryption (scrypt KDF)
// ---------------------------------------------------------------------------

fn bench_encryption(c: &mut Criterion) {
	let mut group = kdf_group(c, "wallet/encryption");
	let password = "benchmark_password_123";

	// Sequential encryption scaling across account counts.
	for count in [1usize, 10] {
		let base = seeded_wallet(count);
		group.bench_with_input(
			BenchmarkId::new("encrypt_sequential", count),
			&count,
			|b, _| {
				b.iter_batched(
					|| base.clone(),
					|mut wallet| {
						wallet.encrypt_accounts(password).unwrap();
						std::hint::black_box(())
					},
					criterion::BatchSize::SmallInput,
				)
			},
		);
	}

	// Parallel encryption for a larger wallet (rayon work-stealing path).
	let base_parallel = seeded_wallet(10);
	group.bench_function("encrypt_parallel_10_accounts", |b| {
		b.iter_batched(
			|| base_parallel.clone(),
			|mut wallet| {
				wallet.encrypt_accounts_parallel(password).unwrap();
				std::hint::black_box(())
			},
			criterion::BatchSize::SmallInput,
		)
	});

	group.finish();
}

fn bench_password_verification(c: &mut Criterion) {
	let mut group = kdf_group(c, "wallet/decryption");

	let mut wallet = seeded_wallet(1);
	wallet.encrypt_accounts("correct_password").unwrap();

	// Correct password exercises the full scrypt decrypt success path.
	group.bench_function("password_verify_correct", |b| {
		b.iter(|| std::hint::black_box(wallet.verify_password("correct_password")))
	});

	// Incorrect password still runs scrypt before failing (constant-time-ish).
	group.bench_function("password_verify_incorrect", |b| {
		b.iter(|| std::hint::black_box(wallet.verify_password("wrong_password")))
	});

	group.finish();
}

// ---------------------------------------------------------------------------
// MACRO-BENCHMARKS: backup / recovery workflows
// ---------------------------------------------------------------------------

fn bench_backup_recovery(c: &mut Criterion) {
	let mut group = kdf_group(c, "wallet/backup_recovery");

	let mut wallet = seeded_wallet(5);
	wallet.encrypt_accounts("test_password").unwrap();

	group.bench_function("backup_5_accounts", |b| {
		b.iter_batched(
			|| {
				// Keep the TempDir alive through the routine; dropping it in the
				// setup closure would delete the dir before the backup writes.
				let temp_dir = TempDir::new().unwrap();
				let backup_path = temp_dir.path().join("benchmark_wallet.json");
				(temp_dir, backup_path)
			},
			|(temp_dir, backup_path)| {
				WalletBackup::backup(&wallet, backup_path).unwrap();
				std::hint::black_box(());
				drop(temp_dir);
			},
			criterion::BatchSize::SmallInput,
		);
	});

	// Prepare a persistent backup for the recovery benchmark.
	let temp_dir = TempDir::new().unwrap();
	let backup_path = temp_dir.path().join("recovery_benchmark.json");
	WalletBackup::backup(&wallet, backup_path.clone()).unwrap();

	group.bench_function("recover_5_accounts", |b| {
		b.iter(|| std::hint::black_box(WalletBackup::recover(backup_path.clone()).unwrap()))
	});

	group.finish();
}

criterion_group!(
	wallet_benches,
	bench_wallet_lifecycle,
	bench_encryption,
	bench_password_verification,
	bench_backup_recovery,
);
criterion_main!(wallet_benches);
