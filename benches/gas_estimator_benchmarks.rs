//! Gas estimation and script-building benchmarks for the Neo Rust SDK.
//!
//! Establishes baselines for fee-calculation accuracy math and the script
//! assembly that feeds the gas estimator under load.
//!
//! Run with:
//! ```bash
//! cargo criterion --bench gas_estimator_benchmarks
//! cargo bench     --bench gas_estimator_benchmarks
//! ```
//!
//! ## Performance Baselines (v3.3.0, x86_64, release profile)
//!
//! | Operation                          | Class | Target p50 | Target p99 |
//! |------------------------------------|-------|-----------:|-----------:|
//! | `accuracy_calculation`             | micro |   ~5 ns    |   <50 ns   |
//! | `calculate_margin/*`               | micro |   ~2 ns    |   <20 ns   |
//! | `build_script_size/1000`           | macro |   ~40 µs   |   <120 µs  |
//! | `simple_script`                    | micro |   ~200 ns  |   <1 µs    |
//! | `complex_script` (100 pushes+pack) | macro |   ~5 µs    |   <20 µs   |
//!
//! Authoritative numbers live in `target/criterion/**/estimates.json`.

use std::time::Duration;

use criterion::{
	criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use neo3::neo_builder::{GasEstimator, ScriptBuilder};
use neo3::neo_types::OpCode;
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
// MICRO-BENCHMARKS: fee arithmetic
// ---------------------------------------------------------------------------

/// Fee estimation accuracy math and margin computation across load levels.
fn bench_gas_calculations(c: &mut Criterion) {
	let mut group = configured_group(c, "gas/calculations", 300);

	group.bench_function("accuracy_calculation", |b| {
		b.iter(|| {
			GasEstimator::calculate_estimation_accuracy(
				std::hint::black_box(1100),
				std::hint::black_box(1000),
			)
		})
	});

	// Fee margin computation under a spread of gas magnitudes (time under load).
	for gas_value in [100i64, 1_000, 10_000, 100_000, 1_000_000, 10_000_000] {
		group.bench_with_input(
			BenchmarkId::new("calculate_margin", gas_value),
			&gas_value,
			|b, &gas| {
				b.iter(|| {
					let base = std::hint::black_box(gas);
					let margin_percent = std::hint::black_box(15i64);
					let margin = (base as f64 * (margin_percent as f64 / 100.0)) as i64;
					std::hint::black_box(base + margin)
				})
			},
		);
	}

	group.finish();
}

// ---------------------------------------------------------------------------
// MICRO / MACRO BENCHMARKS: script assembly feeding fee estimation
// ---------------------------------------------------------------------------

/// Small, fixed-shape scripts (micro) and a packed 100-element script (macro).
fn bench_script_building(c: &mut Criterion) {
	let mut group = configured_group(c, "gas/script_building", 200);

	group.bench_function("simple_script", |b| {
		b.iter(|| {
			ScriptBuilder::new()
				.push_integer(std::hint::black_box(BigInt::from(42)))
				.push_integer(std::hint::black_box(BigInt::from(13)))
				.op_code(&[OpCode::Add])
				.to_bytes()
		})
	});

	group.bench_function("complex_script", |b| {
		b.iter(|| {
			let mut builder = ScriptBuilder::new();
			for i in 0..100 {
				builder.push_integer(std::hint::black_box(BigInt::from(i)));
			}
			builder.push_integer(std::hint::black_box(BigInt::from(100)));
			builder.pack().to_bytes()
		})
	});

	group.bench_function("string_script", |b| {
		let test_string_bytes = "Hello, Neo Blockchain!".as_bytes().to_vec();
		let world_bytes = "World".as_bytes().to_vec();
		b.iter(|| {
			ScriptBuilder::new()
				.push_data(std::hint::black_box(test_string_bytes.clone()))
				.push_data(std::hint::black_box(world_bytes.clone()))
				.op_code(&[OpCode::Cat])
				.to_bytes()
		})
	});

	group.finish();
}

/// Script compilation speed scaling with instruction count. Throughput is
/// reported in elements so per-push cost regressions are visible.
fn bench_script_sizes(c: &mut Criterion) {
	let mut group = configured_group(c, "gas/script_sizes", 100);

	for size in [10u64, 50, 100, 500, 1000, 5000] {
		group.throughput(Throughput::Elements(size));
		group.bench_with_input(
			BenchmarkId::from_parameter(size),
			&size,
			|b, &size| {
				b.iter(|| {
					let mut builder = ScriptBuilder::new();
					for i in 0..size {
						builder.push_integer(std::hint::black_box(BigInt::from(i)));
					}
					builder.to_bytes()
				})
			},
		);
	}

	group.finish();
}

/// Individual and batched opcode emission.
fn bench_opcode_emission(c: &mut Criterion) {
	let mut group = configured_group(c, "gas/opcode_emission", 200);

	group.bench_function("single_opcode", |b| {
		b.iter(|| {
			let opcode = std::hint::black_box(OpCode::Nop);
			ScriptBuilder::new().op_code(&[opcode]).to_bytes()
		})
	});

	group.bench_function("multiple_opcodes", |b| {
		b.iter(|| {
			ScriptBuilder::new()
				.op_code(&[
					std::hint::black_box(OpCode::Push1),
					std::hint::black_box(OpCode::Push2),
					std::hint::black_box(OpCode::Add),
					std::hint::black_box(OpCode::Push3),
					std::hint::black_box(OpCode::Mul),
				])
				.to_bytes()
		})
	});

	group.bench_function("opcode_with_params", |b| {
		let syscall_arg = vec![0u8; 4];
		b.iter(|| {
			ScriptBuilder::new()
				.op_code_with_arg(OpCode::Syscall, std::hint::black_box(syscall_arg.clone()))
				.to_bytes()
		})
	});

	group.finish();
}

criterion_group!(
	gas_benches,
	bench_gas_calculations,
	bench_script_building,
	bench_script_sizes,
	bench_opcode_emission,
);
criterion_main!(gas_benches);
