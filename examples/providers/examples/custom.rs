/// Neo N3 Custom Provider Example
///
/// This example demonstrates how to create custom provider abstractions
/// for Neo N3 blockchain interactions with specialized functionality.
use std::collections::HashMap;
use std::time::Duration;

/// Custom provider trait for Neo N3 operations
trait CustomNeoProvider {
	fn name(&self) -> &str;
	fn configure(&mut self, config: ProviderConfig);
	fn is_healthy(&self) -> bool;
}

/// Configuration for custom providers
#[derive(Clone, Debug)]
struct ProviderConfig {
	timeout_ms: u64,
	max_retries: u32,
	enable_caching: bool,
	endpoints: Vec<String>,
}

impl Default for ProviderConfig {
	fn default() -> Self {
		Self {
			timeout_ms: 5000,
			max_retries: 3,
			enable_caching: true,
			endpoints: vec![
				"https://mainnet1.neo.coz.io:443".to_string(),
				"https://mainnet2.neo.coz.io:443".to_string(),
			],
		}
	}
}

/// Load-balancing provider that distributes requests across multiple endpoints
struct LoadBalancingProvider {
	name: String,
	config: ProviderConfig,
	current_endpoint: usize,
	endpoint_health: HashMap<String, bool>,
}

impl LoadBalancingProvider {
	fn new(name: String) -> Self {
		Self {
			name,
			config: ProviderConfig::default(),
			current_endpoint: 0,
			endpoint_health: HashMap::new(),
		}
	}

	fn get_next_endpoint(&mut self) -> Option<String> {
		if self.config.endpoints.is_empty() {
			return None;
		}

		// Round-robin selection with health checking
		let start_index = self.current_endpoint;
		loop {
			let endpoint = &self.config.endpoints[self.current_endpoint];
			let is_healthy = self.endpoint_health.get(endpoint).unwrap_or(&true);

			self.current_endpoint = (self.current_endpoint + 1) % self.config.endpoints.len();

			if *is_healthy {
				return Some(endpoint.clone());
			}

			if self.current_endpoint == start_index {
				// All endpoints unhealthy, return the first one anyway
				return Some(self.config.endpoints[0].clone());
			}
		}
	}

	fn mark_endpoint_health(&mut self, endpoint: &str, healthy: bool) {
		self.endpoint_health.insert(endpoint.to_string(), healthy);
	}
}

impl CustomNeoProvider for LoadBalancingProvider {
	fn name(&self) -> &str {
		&self.name
	}

	fn configure(&mut self, config: ProviderConfig) {
		self.config = config;
		// Reset health status when reconfiguring
		self.endpoint_health.clear();
	}

	fn is_healthy(&self) -> bool {
		// Provider is healthy if at least one endpoint is healthy
		self.endpoint_health.values().any(|&healthy| healthy) || self.endpoint_health.is_empty()
	}
}

/// Caching provider that caches responses for read-only operations
struct CachingProvider {
	name: String,
	config: ProviderConfig,
	cache: HashMap<String, (String, std::time::Instant)>,
	cache_ttl: Duration,
}

impl CachingProvider {
	fn new(name: String) -> Self {
		Self {
			name,
			config: ProviderConfig::default(),
			cache: HashMap::new(),
			cache_ttl: Duration::from_secs(30),
		}
	}

	fn get_cached(&self, key: &str) -> Option<String> {
		if !self.config.enable_caching {
			return None;
		}

		if let Some((value, timestamp)) = self.cache.get(key) {
			if timestamp.elapsed() < self.cache_ttl {
				return Some(value.clone());
			}
		}
		None
	}

	fn set_cache(&mut self, key: String, value: String) {
		if self.config.enable_caching {
			self.cache.insert(key, (value, std::time::Instant::now()));
		}
	}

	fn is_cacheable_method(&self, method: &str) -> bool {
		// Only cache read-only operations
		matches!(
			method,
			"getversion"
				| "getblockcount"
				| "getpeers" | "getconnectioncount"
				| "getblock" | "getblockheader"
		)
	}
}

impl CustomNeoProvider for CachingProvider {
	fn name(&self) -> &str {
		&self.name
	}

	fn configure(&mut self, config: ProviderConfig) {
		self.config = config;
	}

	fn is_healthy(&self) -> bool {
		true // Caching provider is always healthy
	}
}

/// Monitoring provider that tracks metrics and performance
struct MonitoringProvider {
	name: String,
	config: ProviderConfig,
	request_count: u64,
	error_count: u64,
	total_response_time: Duration,
}

impl MonitoringProvider {
	fn new(name: String) -> Self {
		Self {
			name,
			config: ProviderConfig::default(),
			request_count: 0,
			error_count: 0,
			total_response_time: Duration::ZERO,
		}
	}

	fn record_request(&mut self, duration: Duration, success: bool) {
		self.request_count += 1;
		self.total_response_time += duration;

		if !success {
			self.error_count += 1;
		}
	}

	fn get_metrics(&self) -> ProviderMetrics {
		ProviderMetrics {
			total_requests: self.request_count,
			error_rate: if self.request_count > 0 {
				(self.error_count as f64) / (self.request_count as f64)
			} else {
				0.0
			},
			average_response_time: if self.request_count > 0 {
				self.total_response_time / self.request_count as u32
			} else {
				Duration::ZERO
			},
		}
	}
}

impl CustomNeoProvider for MonitoringProvider {
	fn name(&self) -> &str {
		&self.name
	}

	fn configure(&mut self, config: ProviderConfig) {
		self.config = config;
	}

	fn is_healthy(&self) -> bool {
		let metrics = self.get_metrics();
		metrics.error_rate < 0.1 && metrics.average_response_time < Duration::from_secs(5)
	}
}

/// Metrics structure for monitoring
#[derive(Debug)]
struct ProviderMetrics {
	total_requests: u64,
	error_rate: f64,
	average_response_time: Duration,
}

/// Provider factory for creating different types of providers
struct ProviderFactory;

impl ProviderFactory {
	fn create_load_balancer(endpoints: Vec<String>) -> LoadBalancingProvider {
		let mut provider = LoadBalancingProvider::new("LoadBalancer".to_string());
		provider.configure(ProviderConfig { endpoints, ..ProviderConfig::default() });
		provider
	}

	fn create_caching_provider(cache_ttl_secs: u64) -> CachingProvider {
		let mut provider = CachingProvider::new("CachingProvider".to_string());
		provider.cache_ttl = Duration::from_secs(cache_ttl_secs);
		provider
	}

	fn create_monitoring_provider() -> MonitoringProvider {
		MonitoringProvider::new("MonitoringProvider".to_string())
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("🔧 Neo N3 Custom Provider Example");
	println!("=================================");

	// 1. Load Balancing Provider
	println!("\n1. Load Balancing Provider:");

	let endpoints = vec![
		"https://mainnet1.neo.coz.io:443".to_string(),
		"https://mainnet2.neo.coz.io:443".to_string(),
		"https://mainnet3.neo.coz.io:443".to_string(),
	];

	let mut load_balancer = ProviderFactory::create_load_balancer(endpoints);
	println!("   🌐 Provider: {}", load_balancer.name());
	println!("   🔄 Endpoint selection simulation:");

	for i in 1..=6 {
		if let Some(endpoint) = load_balancer.get_next_endpoint() {
			println!("     Request {i}: {endpoint}");
		}
	}

	// Simulate endpoint failure and recovery
	load_balancer.mark_endpoint_health("https://mainnet2.neo.coz.io:443", false);
	println!("   ⚠️  Marked mainnet2 as unhealthy");

	println!("   🔄 Endpoint selection after failure:");
	for i in 1..=4 {
		if let Some(endpoint) = load_balancer.get_next_endpoint() {
			println!("     Request {i}: {endpoint}");
		}
	}

	// 2. Caching Provider
	println!("\n2. Caching Provider:");

	let mut cache_provider = ProviderFactory::create_caching_provider(60);
	println!("   💾 Provider: {}", cache_provider.name());

	// Simulate caching behavior
	let cache_key = "getblockcount".to_string();
	let response_value = "1234567".to_string();

	println!("   📥 Caching response for 'getblockcount'");
	cache_provider.set_cache(cache_key.clone(), response_value.clone());

	if let Some(cached) = cache_provider.get_cached(&cache_key) {
		println!("   ✅ Cache hit: {cached}");
	}

	// Test cache miss
	if cache_provider.get_cached("nonexistent").is_none() {
		println!("   ❌ Cache miss for unknown key");
	}

	// Test method cacheability
	let methods = vec!["getversion", "sendrawtransaction", "getblock"];
	for method in methods {
		let cacheable = cache_provider.is_cacheable_method(method);
		let icon = if cacheable { "✅" } else { "❌" };
		println!("   {icon} {method} is cacheable: {cacheable}");
	}

	// 3. Monitoring Provider
	println!("\n3. Monitoring Provider:");

	let mut monitor = ProviderFactory::create_monitoring_provider();
	println!("   📊 Provider: {}", monitor.name());

	// Simulate request tracking
	println!("   🔄 Simulating requests:");
	let requests = [
		(Duration::from_millis(150), true),
		(Duration::from_millis(200), true),
		(Duration::from_millis(5000), false), // Timeout
		(Duration::from_millis(100), true),
		(Duration::from_millis(300), true),
	];

	for (i, (duration, success)) in requests.iter().enumerate() {
		monitor.record_request(*duration, *success);
		let status = if *success { "✅" } else { "❌" };
		let request_num = i + 1;
		let duration_ms = duration.as_millis();
		println!("     Request {request_num}: {duration_ms}ms {status}");
	}

	let metrics = monitor.get_metrics();
	println!("   📈 Metrics Summary:");
	println!("     Total requests: {}", metrics.total_requests);
	println!("     Error rate: {:.1}%", metrics.error_rate * 100.0);
	println!("     Average response time: {}ms", metrics.average_response_time.as_millis());
	println!("     Provider healthy: {}", monitor.is_healthy());

	// 4. Provider composition patterns
	println!("\n4. Provider Composition Patterns:");

	println!("   🏗️  Composite Provider Architecture:");
	println!("     ┌─────────────────┐");
	println!("     │   Application   │");
	println!("     └─────────────────┘");
	println!("              │");
	println!("     ┌─────────────────┐");
	println!("     │  Load Balancer  │");
	println!("     └─────────────────┘");
	println!("              │");
	println!("     ┌─────────────────┐");
	println!("     │  Cache Layer    │");
	println!("     └─────────────────┘");
	println!("              │");
	println!("     ┌─────────────────┐");
	println!("     │  Monitor Layer  │");
	println!("     └─────────────────┘");
	println!("              │");
	println!("     ┌─────────────────┐");
	println!("     │   HTTP Client   │");
	println!("     └─────────────────┘");

	// 5. Configuration examples
	println!("\n5. Configuration Examples:");

	let configs = vec![
		(
			"Development",
			ProviderConfig {
				timeout_ms: 1000,
				max_retries: 1,
				enable_caching: false,
				endpoints: vec!["http://localhost:20332".to_string()],
			},
		),
		(
			"Production",
			ProviderConfig {
				timeout_ms: 5000,
				max_retries: 3,
				enable_caching: true,
				endpoints: vec![
					"https://mainnet1.neo.coz.io:443".to_string(),
					"https://mainnet2.neo.coz.io:443".to_string(),
				],
			},
		),
		(
			"Testing",
			ProviderConfig {
				timeout_ms: 10000,
				max_retries: 0,
				enable_caching: false,
				endpoints: vec!["https://testnet.neo.org:443".to_string()],
			},
		),
	];

	for (env, config) in configs {
		println!("   🌍 {env} Environment:");
		println!("     Timeout: {}ms", config.timeout_ms);
		println!("     Max retries: {}", config.max_retries);
		println!("     Caching: {}", config.enable_caching);
		println!("     Endpoints: {}", config.endpoints.len());
	}

	// 6. Best practices
	println!("\n6. Custom Provider Best Practices:");

	let best_practices = vec![
		("Health Checks", "Implement endpoint health monitoring"),
		("Circuit Breaker", "Prevent cascade failures with circuit breakers"),
		("Graceful Degradation", "Fallback to cached data when possible"),
		("Metrics Collection", "Track performance and error rates"),
		("Configuration Management", "Environment-specific configurations"),
		("Connection Pooling", "Reuse HTTP connections efficiently"),
		("Request Deduplication", "Avoid duplicate concurrent requests"),
		("Rate Limiting", "Respect endpoint rate limits"),
	];

	for (practice, description) in best_practices {
		println!("   ✅ {practice}: {description}");
	}

	// 7. Integration examples
	println!("\n7. Integration Code Structure:");

	println!("   ```rust");
	println!("   // Composite provider setup");
	println!("   let provider = CompositeProvider::builder()");
	println!("       .with_load_balancer(endpoints)");
	println!("       .with_caching(Duration::from_secs(30))");
	println!("       .with_monitoring()");
	println!("       .with_circuit_breaker()");
	println!("       .build();");
	println!();
	println!("   // Use with Neo3 client");
	println!("   let client = NeoClient::with_provider(provider);");
	println!("   let result = client.get_block_count().await?;");
	println!("   ```");

	println!("\n🎉 Custom provider example completed!");
	println!("💡 Key concepts demonstrated:");
	println!("   • Load balancing across multiple endpoints");
	println!("   • Response caching for read-only operations");
	println!("   • Request monitoring and metrics collection");
	println!("   • Provider composition and configuration");
	println!("   • Health checking and failure handling");

	Ok(())
}
