use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use neo_gui::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Benchmark tests for Neo GUI performance
/// These tests measure the performance of critical operations

fn benchmark_wallet_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let app_state = AppState {
		wallet_service: Arc::new(services::WalletService::new()),
		network_service: Arc::new(services::NetworkService::new()),
		transaction_service: Arc::new(services::TransactionService::new()),
		settings_service: Arc::new(services::SettingsService::new()),
		rpc_client: Arc::new(std::sync::Mutex::new(None)),
	};

	let mut group = c.benchmark_group("wallet_operations");

	// Benchmark wallet creation
	group.bench_function("create_wallet", |b| {
		b.iter(|| {
			rt.block_on(async {
				let wallet_name = format!("bench_wallet_{}", rand::random::<u32>());
				app_state
					.wallet_service
					.create_wallet(black_box(&wallet_name), black_box("password123"))
					.await
			})
		})
	});

	// Benchmark balance retrieval
	group.bench_function("get_balance", |b| {
		b.iter(|| {
			rt.block_on(async {
				app_state
					.wallet_service
					.get_balance(black_box("test_wallet_id"), black_box(None))
					.await
			})
		})
	});

	// Benchmark transaction history
	group.bench_function("get_transaction_history", |b| {
		b.iter(|| {
			rt.block_on(async {
				app_state
					.wallet_service
					.get_transaction_history(black_box("test_wallet_id"), black_box(None))
					.await
			})
		})
	});

	group.finish();
}

fn benchmark_network_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let app_state = AppState {
		wallet_service: Arc::new(services::WalletService::new()),
		network_service: Arc::new(services::NetworkService::new()),
		transaction_service: Arc::new(services::TransactionService::new()),
		settings_service: Arc::new(services::SettingsService::new()),
		rpc_client: Arc::new(std::sync::Mutex::new(None)),
	};

	let mut group = c.benchmark_group("network_operations");

	// Benchmark network connection
	group.bench_function("connect_to_network", |b| {
		b.iter(|| {
			rt.block_on(async {
				app_state
					.network_service
					.connect_to_network(black_box("http://localhost:10332"))
					.await
			})
		})
	});

	// Benchmark network status
	group.bench_function("get_network_status", |b| {
		// First connect
		rt.block_on(async {
			let _ = app_state.network_service.connect_to_network("http://localhost:10332").await;
		});

		b.iter(|| rt.block_on(async { app_state.network_service.get_network_status().await }))
	});

	// Benchmark block count
	group.bench_function("get_block_count", |b| {
		b.iter(|| rt.block_on(async { app_state.network_service.get_block_count().await }))
	});

	group.finish();
}

fn benchmark_transaction_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let app_state = AppState {
		wallet_service: Arc::new(services::WalletService::new()),
		network_service: Arc::new(services::NetworkService::new()),
		transaction_service: Arc::new(services::TransactionService::new()),
		settings_service: Arc::new(services::SettingsService::new()),
		rpc_client: Arc::new(std::sync::Mutex::new(None)),
	};

	let mut group = c.benchmark_group("transaction_operations");

	// Benchmark transaction sending
	group.bench_function("send_transaction", |b| {
		b.iter(|| {
			rt.block_on(async {
				app_state
					.transaction_service
					.send_transaction(
						black_box("NX8GreRFGFK5wpGMWetpX93HmtrezGogzk"),
						black_box("NX8GreRFGFK5wpGMWetpX93HmtrezGogzl"),
						black_box("NEO"),
						black_box("1"),
						black_box(None),
					)
					.await
			})
		})
	});

	// Benchmark transaction retrieval
	group.bench_function("get_transaction", |b| {
		b.iter(|| {
			rt.block_on(async {
				let tx_id = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12";
				app_state.transaction_service.get_transaction(black_box(tx_id)).await
			})
		})
	});

	// Benchmark gas estimation
	group.bench_function("estimate_gas", |b| {
		b.iter(|| {
			rt.block_on(async {
				app_state
					.transaction_service
					.estimate_gas(
						black_box("NX8GreRFGFK5wpGMWetpX93HmtrezGogzk"),
						black_box("NX8GreRFGFK5wpGMWetpX93HmtrezGogzl"),
						black_box("NEO"),
						black_box("1"),
					)
					.await
			})
		})
	});

	group.finish();
}

fn benchmark_settings_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let app_state = AppState {
		wallet_service: Arc::new(services::WalletService::new()),
		network_service: Arc::new(services::NetworkService::new()),
		transaction_service: Arc::new(services::TransactionService::new()),
		settings_service: Arc::new(services::SettingsService::new()),
		rpc_client: Arc::new(std::sync::Mutex::new(None)),
	};

	let mut group = c.benchmark_group("settings_operations");

	// Benchmark settings retrieval
	group.bench_function("get_settings", |b| {
		b.iter(|| rt.block_on(async { app_state.settings_service.get_settings().await }))
	});

	// Benchmark settings update
	group.bench_function("update_settings", |b| {
		b.iter(|| {
			rt.block_on(async {
				let mut settings = app_state.settings_service.get_settings().await.unwrap();
				settings.auto_lock_timeout = black_box(300 + rand::random::<u32>() % 1000);
				app_state.settings_service.update_settings(black_box(settings)).await
			})
		})
	});

	// Benchmark endpoint operations
	group.bench_function("add_endpoint", |b| {
		b.iter(|| {
			rt.block_on(async {
				let endpoint = services::settings::NetworkEndpoint {
					name: format!("Bench Endpoint {}", rand::random::<u32>()),
					url: format!("http://localhost:{}", 10332 + rand::random::<u16>() % 1000),
					network_type: "testnet".to_string(),
					is_default: false,
				};
				app_state.settings_service.add_endpoint(black_box(endpoint)).await
			})
		})
	});

	group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let app_state = Arc::new(AppState {
		wallet_service: Arc::new(services::WalletService::new()),
		network_service: Arc::new(services::NetworkService::new()),
		transaction_service: Arc::new(services::TransactionService::new()),
		settings_service: Arc::new(services::SettingsService::new()),
		rpc_client: Arc::new(std::sync::Mutex::new(None)),
	});

	let mut group = c.benchmark_group("concurrent_operations");

	// Benchmark concurrent wallet operations
	for concurrency in [1, 5, 10, 20].iter() {
		group.bench_with_input(
			BenchmarkId::new("concurrent_wallet_ops", concurrency),
			concurrency,
			|b, &concurrency| {
				b.iter(|| {
					rt.block_on(async {
						let mut handles = vec![];

						for i in 0..concurrency {
							let app_state_clone = Arc::clone(&app_state);
							let handle = tokio::spawn(async move {
								let wallet_name =
									format!("concurrent_wallet_{}_{}", concurrency, i);
								app_state_clone
									.wallet_service
									.create_wallet(&wallet_name, "password")
									.await
							});
							handles.push(handle);
						}

						for handle in handles {
							let _ = handle.await;
						}
					})
				})
			},
		);
	}

	// Benchmark concurrent transaction operations
	for concurrency in [1, 5, 10, 20].iter() {
		group.bench_with_input(
			BenchmarkId::new("concurrent_transaction_ops", concurrency),
			concurrency,
			|b, &concurrency| {
				b.iter(|| {
					rt.block_on(async {
						let mut handles = vec![];

						for i in 0..concurrency {
							let app_state_clone = Arc::clone(&app_state);
							let handle = tokio::spawn(async move {
								app_state_clone
									.transaction_service
									.send_transaction(
										"NX8GreRFGFK5wpGMWetpX93HmtrezGogzk",
										"NX8GreRFGFK5wpGMWetpX93HmtrezGogzl",
										"NEO",
										&format!("{}", i + 1),
										None,
									)
									.await
							});
							handles.push(handle);
						}

						for handle in handles {
							let _ = handle.await;
						}
					})
				})
			},
		);
	}

	group.finish();
}

fn benchmark_memory_operations(c: &mut Criterion) {
	let rt = Runtime::new().unwrap();

	let app_state = AppState {
		wallet_service: Arc::new(services::WalletService::new()),
		network_service: Arc::new(services::NetworkService::new()),
		transaction_service: Arc::new(services::TransactionService::new()),
		settings_service: Arc::new(services::SettingsService::new()),
		rpc_client: Arc::new(std::sync::Mutex::new(None)),
	};

	let mut group = c.benchmark_group("memory_operations");

	// Benchmark memory usage with many operations
	group.bench_function("memory_intensive_operations", |b| {
		b.iter(|| {
			rt.block_on(async {
				// Perform many operations to test memory usage
				for i in 0..black_box(100) {
					let wallet_name = format!("memory_wallet_{}", i);
					let _ = app_state.wallet_service.create_wallet(&wallet_name, "password").await;
					let _ = app_state.wallet_service.get_balance(&wallet_name, None).await;

					let tx_id = format!("0x{:064}", i);
					let _ = app_state.transaction_service.get_transaction(&tx_id).await;
				}
			})
		})
	});

	group.finish();
}

criterion_group!(
	benches,
	benchmark_wallet_operations,
	benchmark_network_operations,
	benchmark_transaction_operations,
	benchmark_settings_operations,
	benchmark_concurrent_operations,
	benchmark_memory_operations
);

criterion_main!(benches);
