use std::{
	collections::VecDeque,
	sync::{Arc, Mutex},
};

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::common::JsonRpcError;
use crate::neo_clients::{JsonRpcProvider, ProviderError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchKind {
	Any,
	Partial,
	Exact,
}

#[derive(Clone, Debug)]
enum ParamsMatcher {
	Any,
	Partial(Value),
	Exact(Value),
}

impl ParamsMatcher {
	fn kind(&self) -> MatchKind {
		match self {
			ParamsMatcher::Any => MatchKind::Any,
			ParamsMatcher::Partial(_) => MatchKind::Partial,
			ParamsMatcher::Exact(_) => MatchKind::Exact,
		}
	}

	fn matches(&self, actual: &Value) -> bool {
		match self {
			ParamsMatcher::Any => true,
			ParamsMatcher::Exact(expected) => expected == actual,
			ParamsMatcher::Partial(expected) => json_partial_match(expected, actual),
		}
	}
}

#[derive(Clone, Debug)]
enum MethodMatcher {
	Any,
	Exact(String),
}

impl MethodMatcher {
	fn matches(&self, actual: &str) -> bool {
		match self {
			MethodMatcher::Any => true,
			MethodMatcher::Exact(expected) => expected == actual,
		}
	}

	fn kind(&self) -> MatchKind {
		match self {
			MethodMatcher::Any => MatchKind::Any,
			MethodMatcher::Exact(_) => MatchKind::Exact,
		}
	}
}

#[derive(Clone, Debug)]
pub enum MockResponse {
	Result(Value),
	Error(JsonRpcError),
}

#[derive(Clone, Debug)]
struct MockRule {
	method: MethodMatcher,
	params: ParamsMatcher,
	response: MockResponse,
}

#[derive(Clone, Debug, Default)]
pub struct MockProvider {
	rules: Arc<Mutex<Vec<MockRule>>>,
	requests: Arc<Mutex<VecDeque<(String, Value)>>>,
}

impl MockProvider {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push_result(&self, method: impl Into<String>, result: Value) {
		self.push_rule(method.into(), ParamsMatcher::Any, MockResponse::Result(result));
	}

	pub fn push_result_with_params(&self, method: impl Into<String>, params: Value, result: Value) {
		self.push_rule(method.into(), ParamsMatcher::Exact(params), MockResponse::Result(result));
	}

	pub fn push_result_with_partial_params(
		&self,
		method: impl Into<String>,
		params: Value,
		result: Value,
	) {
		self.push_rule(method.into(), ParamsMatcher::Partial(params), MockResponse::Result(result));
	}

	pub fn push_error_any(&self, error: JsonRpcError) {
		let mut rules = self.rules.lock().unwrap_or_else(|e| e.into_inner());
		rules.push(MockRule {
			method: MethodMatcher::Any,
			params: ParamsMatcher::Any,
			response: MockResponse::Error(error),
		});
	}

	pub fn push_error(&self, method: impl Into<String>, error: JsonRpcError) {
		self.push_rule(method.into(), ParamsMatcher::Any, MockResponse::Error(error));
	}

	fn push_rule(&self, method: String, params: ParamsMatcher, response: MockResponse) {
		let mut rules = self.rules.lock().unwrap_or_else(|e| e.into_inner());
		rules.push(MockRule { method: MethodMatcher::Exact(method), params, response });
	}

	pub fn take_requests(&self) -> VecDeque<(String, Value)> {
		self.requests.lock().unwrap_or_else(|e| e.into_inner()).drain(..).collect()
	}

	pub fn assert_request<T: Serialize>(
		&self,
		method: &str,
		params: T,
	) -> Result<(), ProviderError> {
		let expected = serde_json::to_value(params)?;
		let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
		let (actual_method, actual_params) = requests
			.pop_front()
			.ok_or_else(|| ProviderError::CustomError("No recorded mock requests".to_string()))?;
		if actual_method != method {
			return Err(ProviderError::CustomError(format!(
				"Expected method {method}, got {actual_method}"
			)));
		}
		if actual_params != expected {
			return Err(ProviderError::CustomError(format!(
				"Expected params {expected}, got {actual_params}"
			)));
		}
		Ok(())
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(? Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcProvider for MockProvider {
	type Error = ProviderError;

	async fn fetch<T, R>(&self, method: &str, params: T) -> Result<R, ProviderError>
	where
		T: std::fmt::Debug + Serialize + Send + Sync,
		R: DeserializeOwned + Send,
	{
		let params_value = serde_json::to_value(params)?;
		{
			let mut requests = self.requests.lock().unwrap_or_else(|e| e.into_inner());
			requests.push_back((method.to_string(), params_value.clone()));
		}

		let rule = {
			let rules = self.rules.lock().unwrap_or_else(|e| e.into_inner());
			select_best_rule(&rules, method, &params_value).cloned()
		};

		let Some(rule) = rule else {
			return Err(ProviderError::CustomError(format!(
				"No mock response configured for method {method} with params {params_value}"
			)));
		};

		match rule.response {
			MockResponse::Result(value) => serde_json::from_value(value).map_err(Into::into),
			MockResponse::Error(error) => Err(ProviderError::JsonRpcError(error)),
		}
	}
}

fn select_best_rule<'a>(
	rules: &'a [MockRule],
	method: &str,
	params: &Value,
) -> Option<&'a MockRule> {
	rules
		.iter()
		.filter(|rule| rule.method.matches(method) && rule.params.matches(params))
		.max_by_key(|rule| (rule.method.kind() as u8, rule.params.kind() as u8))
}

fn json_partial_match(expected: &Value, actual: &Value) -> bool {
	match (expected, actual) {
		(Value::Object(expected), Value::Object(actual)) => expected.iter().all(|(key, value)| {
			actual
				.get(key)
				.is_some_and(|actual_value| json_partial_match(value, actual_value))
		}),
		(Value::Array(expected), Value::Array(actual)) => {
			expected.len() <= actual.len()
				&& expected.iter().zip(actual.iter()).all(|(expected_item, actual_item)| {
					json_partial_match(expected_item, actual_item)
				})
		},
		_ => expected == actual,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::neo_clients::JsonRpcProvider;
	use serde_json::json;

	#[test]
	fn test_json_partial_match_object_subset() {
		let expected = json!({"a": 1});
		let actual = json!({"a": 1, "b": 2});
		assert!(json_partial_match(&expected, &actual));

		let expected = json!({"a": 2});
		assert!(!json_partial_match(&expected, &actual));
	}

	#[test]
	fn test_json_partial_match_array_prefix() {
		let expected = json!([1, 2]);
		let actual = json!([1, 2, 3]);
		assert!(json_partial_match(&expected, &actual));

		let expected = json!([1, 3]);
		assert!(!json_partial_match(&expected, &actual));

		let expected = json!([1, 2, 3, 4]);
		assert!(!json_partial_match(&expected, &actual));
	}

	#[tokio::test]
	async fn test_mock_provider_selects_most_specific_rule() {
		let provider = MockProvider::new();

		provider.push_result("getblockcount", json!(1));
		provider.push_result_with_partial_params("getblockcount", json!([1]), json!(2));
		provider.push_result_with_params("getblockcount", json!([1, 2]), json!(3));

		let exact: i32 = provider.fetch("getblockcount", vec![1, 2]).await.unwrap();
		assert_eq!(exact, 3);

		let partial: i32 = provider.fetch("getblockcount", vec![1, 9, 9]).await.unwrap();
		assert_eq!(partial, 2);

		let fallback: i32 = provider.fetch("getblockcount", vec![9]).await.unwrap();
		assert_eq!(fallback, 1);
	}

	#[tokio::test]
	async fn test_mock_provider_records_requests() {
		let provider = MockProvider::new();
		provider.push_result_with_params("getversion", json!([]), json!({"useragent": "test"}));

		let _: Value = provider.fetch("getversion", Vec::<u32>::new()).await.unwrap();

		provider.assert_request("getversion", Vec::<u32>::new()).unwrap();
		assert!(provider.take_requests().is_empty());
	}
}
