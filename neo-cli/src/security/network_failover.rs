use crate::errors::CliError;
use neo3::prelude::{HttpProvider, RpcClient};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// RPC endpoint health status
#[derive(Debug, Clone)]
pub struct EndpointHealth {
    pub url: String,
    pub is_healthy: bool,
    pub response_time: Duration,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_check: Instant,
    pub last_error: Option<String>,
}

impl EndpointHealth {
    pub fn new(url: String) -> Self {
        Self {
            url,
            is_healthy: true,
            response_time: Duration::from_millis(0),
            failure_count: 0,
            success_count: 0,
            last_check: Instant::now(),
            last_error: None,
        }
    }

    pub fn success(&mut self, response_time: Duration) {
        self.is_healthy = true;
        self.response_time = response_time;
        self.success_count += 1;
        self.failure_count = 0;
        self.last_check = Instant::now();
        self.last_error = None;
    }

    pub fn failure(&mut self, error: String) {
        self.failure_count += 1;
        self.last_check = Instant::now();
        self.last_error = Some(error);
        
        // Mark unhealthy after 3 consecutive failures
        if self.failure_count >= 3 {
            self.is_healthy = false;
        }
    }

    pub fn score(&self) -> f64 {
        if !self.is_healthy {
            return 0.0;
        }

        let success_rate = if self.success_count + self.failure_count > 0 {
            self.success_count as f64 / (self.success_count + self.failure_count) as f64
        } else {
            0.0
        };

        let response_score = 1.0 / (1.0 + self.response_time.as_millis() as f64 / 1000.0);
        
        success_rate * 0.7 + response_score * 0.3
    }
}

/// Network failover configuration
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    pub health_check_interval: Duration,
    pub request_timeout: Duration,
    pub max_retries: u32,
    pub failover_threshold: u32,
    pub recovery_interval: Duration,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(30),
            request_timeout: Duration::from_secs(10),
            max_retries: 3,
            failover_threshold: 3,
            recovery_interval: Duration::from_secs(60),
        }
    }
}

/// Network failover manager for RPC endpoints
pub struct NetworkFailover {
    endpoints: Arc<RwLock<Vec<EndpointHealth>>>,
    current_index: Arc<RwLock<usize>>,
    config: FailoverConfig,
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
}

impl NetworkFailover {
    pub fn new(endpoints: Vec<String>, config: FailoverConfig) -> Self {
        let health_endpoints = endpoints
            .into_iter()
            .map(EndpointHealth::new)
            .collect();

        Self {
            endpoints: Arc::new(RwLock::new(health_endpoints)),
            current_index: Arc::new(RwLock::new(0)),
            config,
            health_check_handle: None,
        }
    }

    /// Start health monitoring
    pub fn start_health_monitoring(&mut self) {
        let endpoints = Arc::clone(&self.endpoints);
        let config = self.config.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(config.health_check_interval).await;
                Self::check_all_endpoints(endpoints.clone(), &config).await;
            }
        });

        self.health_check_handle = Some(handle);
    }

    /// Stop health monitoring
    pub fn stop_health_monitoring(&mut self) {
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
        }
    }

    /// Get the current best endpoint
    pub fn get_best_endpoint(&self) -> Result<String, CliError> {
        let endpoints = self.endpoints.read().unwrap();
        
        // Find the best healthy endpoint based on score
        let best = endpoints
            .iter()
            .filter(|e| e.is_healthy)
            .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap());

        match best {
            Some(endpoint) => Ok(endpoint.url.clone()),
            None => {
                // All endpoints unhealthy, try the first one
                endpoints
                    .first()
                    .map(|e| e.url.clone())
                    .ok_or_else(|| CliError::Network("No RPC endpoints available".to_string()))
            }
        }
    }

    /// Execute RPC call with automatic failover
    pub async fn execute_with_failover<T, F, Fut>(&self, operation: F) -> Result<T, CliError>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<T, CliError>>,
    {
        let mut last_error = None;
        let mut attempts = 0;

        while attempts < self.config.max_retries {
            let endpoint = self.get_best_endpoint()?;
            let start = Instant::now();

            match timeout(self.config.request_timeout, operation(endpoint.clone())).await {
                Ok(Ok(result)) => {
                    // Update health on success
                    self.update_endpoint_health(&endpoint, true, start.elapsed(), None);
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    last_error = Some(e.clone());
                    self.update_endpoint_health(&endpoint, false, start.elapsed(), Some(e.to_string()));
                    
                    // Try failover to next endpoint
                    self.failover_to_next();
                }
                Err(_) => {
                    last_error = Some(CliError::Timeout("RPC request timed out".to_string()));
                    self.update_endpoint_health(&endpoint, false, start.elapsed(), Some("Timeout".to_string()));
                    
                    // Try failover to next endpoint
                    self.failover_to_next();
                }
            }

            attempts += 1;
        }

        Err(last_error.unwrap_or_else(|| {
            CliError::Network("All RPC endpoints failed".to_string())
        }))
    }

    /// Update endpoint health status
    fn update_endpoint_health(&self, url: &str, success: bool, response_time: Duration, error: Option<String>) {
        let mut endpoints = self.endpoints.write().unwrap();
        
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            if success {
                endpoint.success(response_time);
            } else {
                endpoint.failure(error.unwrap_or_else(|| "Unknown error".to_string()));
            }
        }
    }

    /// Failover to the next available endpoint
    fn failover_to_next(&self) {
        let endpoints = self.endpoints.read().unwrap();
        let mut current = self.current_index.write().unwrap();
        
        // Find next healthy endpoint
        let start_index = *current;
        loop {
            *current = (*current + 1) % endpoints.len();
            
            if endpoints[*current].is_healthy {
                log::info!("Failing over to endpoint: {}", endpoints[*current].url);
                break;
            }

            // Prevent infinite loop
            if *current == start_index {
                log::warn!("No healthy endpoints available for failover");
                break;
            }
        }
    }

    /// Check health of all endpoints
    async fn check_all_endpoints(endpoints: Arc<RwLock<Vec<EndpointHealth>>>, config: &FailoverConfig) {
        let endpoint_urls: Vec<String> = {
            let eps = endpoints.read().unwrap();
            eps.iter().map(|e| e.url.clone()).collect()
        };

        for url in endpoint_urls {
            let health = Self::check_endpoint_health(&url, config).await;
            
            let mut eps = endpoints.write().unwrap();
            if let Some(endpoint) = eps.iter_mut().find(|e| e.url == url) {
                if health.0 {
                    endpoint.success(health.1);
                } else {
                    endpoint.failure(health.2);
                }
            }
        }
    }

    /// Check health of a single endpoint
    async fn check_endpoint_health(url: &str, config: &FailoverConfig) -> (bool, Duration, String) {
        let start = Instant::now();
        
        match timeout(config.request_timeout, Self::ping_endpoint(url)).await {
            Ok(Ok(_)) => (true, start.elapsed(), String::new()),
            Ok(Err(e)) => (false, start.elapsed(), e.to_string()),
            Err(_) => (false, config.request_timeout, "Timeout".to_string()),
        }
    }

    /// Ping an endpoint to check if it's alive
    async fn ping_endpoint(url: &str) -> Result<(), CliError> {
        let provider = HttpProvider::new(url)
            .map_err(|e| CliError::Network(format!("Invalid RPC URL: {}", e)))?;
        
        let client = RpcClient::new(provider);
        
        // Simple ping using get_version
        client.get_version()
            .await
            .map_err(|e| CliError::Network(format!("RPC ping failed: {}", e)))?;
        
        Ok(())
    }

    /// Get health statistics for all endpoints
    pub fn get_health_stats(&self) -> Vec<EndpointHealth> {
        self.endpoints.read().unwrap().clone()
    }

    /// Add a new endpoint
    pub fn add_endpoint(&self, url: String) {
        let mut endpoints = self.endpoints.write().unwrap();
        if !endpoints.iter().any(|e| e.url == url) {
            endpoints.push(EndpointHealth::new(url));
        }
    }

    /// Remove an endpoint
    pub fn remove_endpoint(&self, url: &str) {
        let mut endpoints = self.endpoints.write().unwrap();
        endpoints.retain(|e| e.url != url);
    }

    /// Reset all endpoint health stats
    pub fn reset_health_stats(&self) {
        let mut endpoints = self.endpoints.write().unwrap();
        for endpoint in endpoints.iter_mut() {
            endpoint.is_healthy = true;
            endpoint.failure_count = 0;
            endpoint.success_count = 0;
            endpoint.last_error = None;
        }
    }
}

/// Builder for NetworkFailover
pub struct NetworkFailoverBuilder {
    endpoints: Vec<String>,
    config: FailoverConfig,
}

impl NetworkFailoverBuilder {
    pub fn new() -> Self {
        Self {
            endpoints: Vec::new(),
            config: FailoverConfig::default(),
        }
    }

    pub fn add_endpoint(mut self, url: String) -> Self {
        self.endpoints.push(url);
        self
    }

    pub fn add_endpoints(mut self, urls: Vec<String>) -> Self {
        self.endpoints.extend(urls);
        self
    }

    pub fn with_config(mut self, config: FailoverConfig) -> Self {
        self.config = config;
        self
    }

    pub fn health_check_interval(mut self, interval: Duration) -> Self {
        self.config.health_check_interval = interval;
        self
    }

    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.config.request_timeout = timeout;
        self
    }

    pub fn build(self) -> NetworkFailover {
        NetworkFailover::new(self.endpoints, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_health_scoring() {
        let mut health = EndpointHealth::new("http://localhost:8080".to_string());
        
        health.success(Duration::from_millis(100));
        health.success(Duration::from_millis(150));
        
        assert!(health.score() > 0.5);
        assert!(health.is_healthy);
        
        health.failure("Connection refused".to_string());
        health.failure("Connection refused".to_string());
        health.failure("Connection refused".to_string());
        
        assert!(!health.is_healthy);
        assert_eq!(health.score(), 0.0);
    }

    #[tokio::test]
    async fn test_failover_manager() {
        let endpoints = vec![
            "http://localhost:8080".to_string(),
            "http://localhost:8081".to_string(),
            "http://localhost:8082".to_string(),
        ];

        let failover = NetworkFailover::new(endpoints, FailoverConfig::default());
        
        let best = failover.get_best_endpoint();
        assert!(best.is_ok());
    }
}