use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use neo3::neo_builder::{GasEstimator, ScriptBuilder};
use neo3::neo_types::OpCode;

fn benchmark_script_building(c: &mut Criterion) {
	let mut group = c.benchmark_group("script_building");

	// Benchmark simple script building
	group.bench_function("simple_script", |b| {
		b.iter(|| {
			ScriptBuilder::new()
				.push_integer(black_box(42))
				.push_integer(black_box(13))
				.emit(OpCode::Add)
				.to_bytes()
		})
	});

	// Benchmark complex script building
	group.bench_function("complex_script", |b| {
		b.iter(|| {
			let mut builder = ScriptBuilder::new();
			for i in 0..100 {
				builder = builder.push_integer(black_box(i));
			}
			builder.emit(OpCode::Pack).to_bytes()
		})
	});

	// Benchmark script with strings
	group.bench_function("string_script", |b| {
		let test_string = "Hello, Neo Blockchain!";
		b.iter(|| {
			ScriptBuilder::new()
				.push_string(black_box(test_string.to_string()))
				.push_string(black_box("World".to_string()))
				.emit(OpCode::Cat)
				.to_bytes()
		})
	});

	group.finish();
}

fn benchmark_gas_calculations(c: &mut Criterion) {
	let mut group = c.benchmark_group("gas_calculations");

	// Benchmark accuracy calculation
	group.bench_function("accuracy_calculation", |b| {
		b.iter(|| GasEstimator::calculate_estimation_accuracy(black_box(1100), black_box(1000)))
	});

	// Benchmark with different gas values
	for gas_value in [100, 1_000, 10_000, 100_000, 1_000_000].iter() {
		group.bench_with_input(
			BenchmarkId::new("calculate_margin", gas_value),
			gas_value,
			|b, &gas| {
				b.iter(|| {
					let base = black_box(gas);
					let margin_percent = black_box(15);
					let margin = (base as f64 * (margin_percent as f64 / 100.0)) as i64;
					base + margin
				})
			},
		);
	}

	group.finish();
}

fn benchmark_script_sizes(c: &mut Criterion) {
	let mut group = c.benchmark_group("script_sizes");

	// Benchmark different script sizes
	for size in [10, 50, 100, 500, 1000].iter() {
		group.bench_with_input(BenchmarkId::new("build_script_size", size), size, |b, &size| {
			b.iter(|| {
				let mut builder = ScriptBuilder::new();
				for i in 0..size {
					builder = builder.push_integer(black_box(i as i64));
				}
				builder.to_bytes()
			})
		});
	}

	group.finish();
}

fn benchmark_opcode_emission(c: &mut Criterion) {
	let mut group = c.benchmark_group("opcode_emission");

	// Benchmark single opcode emission
	group.bench_function("single_opcode", |b| {
		b.iter(|| ScriptBuilder::new().emit(black_box(OpCode::Nop)).to_bytes())
	});

	// Benchmark multiple opcode emission
	group.bench_function("multiple_opcodes", |b| {
		b.iter(|| {
			ScriptBuilder::new()
				.emit(black_box(OpCode::Push1))
				.emit(black_box(OpCode::Push2))
				.emit(black_box(OpCode::Add))
				.emit(black_box(OpCode::Push3))
				.emit(black_box(OpCode::Mul))
				.to_bytes()
		})
	});

	// Benchmark opcode with parameters
	group.bench_function("opcode_with_params", |b| {
		let contract_hash = [0u8; 20];
		b.iter(|| {
			ScriptBuilder::new()
				.emit_push(&black_box(contract_hash))
				.emit(OpCode::Appcall)
				.to_bytes()
		})
	});

	group.finish();
}

criterion_group!(
	benches,
	benchmark_script_building,
	benchmark_gas_calculations,
	benchmark_script_sizes,
	benchmark_opcode_emission
);
criterion_main!(benches);
