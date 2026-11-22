// Copyright (c) 2023-2025 R3E Network
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Client for interacting with NeoFS.
use base64::Engine;

use crate::{
	neo_fs::{
		AccessPermission, BearerToken, Container, ContainerId, MultipartUpload,
		MultipartUploadResult, NeoFSError, NeoFSResult, NeoFSService, Object, ObjectId, OwnerId,
		Part, SessionToken,
	},
	neo_protocol::Account,
};
use async_trait::async_trait;
use base64;
use reqwest::{
	Client,
	header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde_json::{Value, json};
use std::time::Duration;
use tracing::warn;

pub use super::{
	DEFAULT_MAINNET_ENDPOINT, DEFAULT_MAINNET_HTTP_GATEWAY, DEFAULT_MAINNET_REST_API,
	DEFAULT_TESTNET_ENDPOINT, DEFAULT_TESTNET_HTTP_GATEWAY, DEFAULT_TESTNET_REST_API, NeoFSAuth,
	NeoFSConfig,
};

/// Client for interacting with NeoFS
#[derive(Debug, Clone)]
pub struct NeoFSClient {
	config: NeoFSConfig,
	account: Option<Account>,
	http_client: Client,
	base_url: String,
}

impl Default for NeoFSClient {
	fn default() -> Self {
		Self::new(NeoFSConfig {
			endpoint: DEFAULT_MAINNET_REST_API.to_string(),
			auth: None,
			timeout_sec: 10,
			insecure: false,
		})
	}
}

impl NeoFSClient {
	fn parse_container_ids_response(response: &Value) -> NeoFSResult<Vec<ContainerId>> {
		let containers =
			response.get("containers").and_then(|v| v.as_array()).ok_or_else(|| {
				NeoFSError::UnexpectedResponse(
					"Missing or invalid 'containers' field in response".to_string(),
				)
			})?;

		containers
			.iter()
			.map(|value| {
				let id = value.get("containerId").and_then(|id| id.as_str()).ok_or_else(|| {
					NeoFSError::UnexpectedResponse(
						"Missing or invalid containerId in response".to_string(),
					)
				})?;
				Ok(ContainerId(id.to_string()))
			})
			.collect()
	}

	fn parse_object_ids_response(response: &Value) -> NeoFSResult<Vec<ObjectId>> {
		let objects = response.get("objects").and_then(|v| v.as_array()).ok_or_else(|| {
			NeoFSError::UnexpectedResponse(
				"Missing or invalid 'objects' field in response".to_string(),
			)
		})?;

		objects
			.iter()
			.map(|value| {
				let id = value.get("objectId").and_then(|id| id.as_str()).ok_or_else(|| {
					NeoFSError::UnexpectedResponse(
						"Missing or invalid objectId in response".to_string(),
					)
				})?;
				Ok(ObjectId(id.to_string()))
			})
			.collect()
	}

	/// Creates a new NeoFS client with the given configuration
	pub fn new(config: NeoFSConfig) -> Self {
		let http_client = {
			let mut builder = Client::builder();

			#[cfg(not(target_arch = "wasm32"))]
			{
				if config.insecure {
					builder = builder
						.danger_accept_invalid_certs(true)
						.danger_accept_invalid_hostnames(true);
				}
			}

			builder.build().unwrap_or_else(|err| {
				warn!("Failed to build NeoFS HTTP client: {err}; falling back to default client");
				Client::new()
			})
		};
		let base_url = if config.endpoint.starts_with("http") {
			config.endpoint.clone()
		} else {
			// Convert gRPC endpoint to the REST API base URL.
			if config.endpoint.contains("mainnet") {
				DEFAULT_MAINNET_REST_API.to_string()
			} else {
				DEFAULT_TESTNET_REST_API.to_string()
			}
		};

		Self { config, account: None, http_client, base_url }
	}

	/// Sets the account to use for authentication
	#[must_use]
	pub fn with_account(mut self, account: Account) -> Self {
		self.account = Some(account);
		self
	}

	/// Gets the account's owner ID
	pub fn get_owner_id(&self) -> NeoFSResult<OwnerId> {
		if let Some(account) = &self.account {
			let pubkey = account
				.get_public_key()
				.ok_or(NeoFSError::AuthenticationError("No public key available".to_string()))?
				.to_string();

			Ok(OwnerId(pubkey))
		} else if let Some(auth) = &self.config.auth {
			// Best-effort fallback for environments where a full Account is not available.
			// Note: Real NeoFS owner IDs are derived from keys; treat this as an identifier only.
			Ok(OwnerId(auth.wallet_address.clone()))
		} else {
			Err(NeoFSError::AuthenticationError(
				"No account provided for authentication".to_string(),
			))
		}
	}

	/// Creates HTTP headers for authenticated requests
	fn create_auth_headers(&self) -> NeoFSResult<HeaderMap> {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

		if let Some(auth) = &self.config.auth
			&& auth.private_key.is_some()
		{
			return Err(NeoFSError::UnsupportedOperation(
					"raw private-key request signing is not supported by the REST gateway client; use an Account-backed native NeoFS client for signed container, ACL, or token operations"
						.to_string(),
				));
		}

		Ok(headers)
	}

	/// Makes an HTTP request to the NeoFS REST API
	async fn make_request(
		&self,
		method: &str,
		endpoint: &str,
		body: Option<Value>,
	) -> NeoFSResult<Value> {
		let url = format!("{}/v1/{}", self.base_url.trim_end_matches('/'), endpoint);
		let headers = self.create_auth_headers()?;

		let mut request = match method {
			"GET" => self.http_client.get(&url),
			"POST" => self.http_client.post(&url),
			"PUT" => self.http_client.put(&url),
			"DELETE" => self.http_client.delete(&url),
			_ => {
				return Err(NeoFSError::InvalidArgument(format!(
					"Unsupported HTTP method: {}",
					method
				)));
			},
		};

		request = request.headers(headers);
		if self.config.timeout_sec > 0 {
			request = request.timeout(Duration::from_secs(self.config.timeout_sec));
		}

		if let Some(json_body) = body {
			request = request.json(&json_body);
		}

		let response = request
			.send()
			.await
			.map_err(|e| NeoFSError::ConnectionError(format!("HTTP request failed: {}", e)))?;

		if !response.status().is_success() {
			return Err(NeoFSError::UnexpectedResponse(format!(
				"HTTP error: {}",
				response.status()
			)));
		}

		let json: Value = response.json().await.map_err(|e| {
			NeoFSError::SerializationError(format!("Failed to parse JSON response: {}", e))
		})?;

		Ok(json)
	}

	// MULTIPART UPLOAD OPERATIONS

	/// Initializes a multipart upload
	pub async fn init_multipart_upload(
		&self,
		container_id: &ContainerId,
		object: &Object,
		part_size: u64,
	) -> NeoFSResult<MultipartUpload> {
		let owner_id = self.get_owner_id()?;

		// Create multipart upload request
		let request_body = json!({
			"multipartUpload": {
				"containerId": container_id.0,
				"ownerId": owner_id.0,
				"attributes": object.attributes,
				"partSize": part_size,
				"maxParts": 10000
			}
		});

		let response = self.make_request("POST", "multipart/init", Some(request_body)).await?;

		let upload_id = response.get("uploadId").and_then(|v| v.as_str()).ok_or_else(|| {
			NeoFSError::UnexpectedResponse("Missing uploadId in response".to_string())
		})?;

		Ok(MultipartUpload {
			id: None,
			container_id: container_id.clone(),
			owner_id,
			upload_id: upload_id.to_string(),
			attributes: object.attributes.clone(),
			part_size,
			max_parts: 10000,
		})
	}

	/// Uploads a part of a multipart upload
	pub async fn upload_part(&self, upload: &MultipartUpload, part: Part) -> NeoFSResult<()> {
		let request_body = json!({
			"uploadId": upload.upload_id,
			"partNumber": part.part_number,
			"data": base64::engine::general_purpose::STANDARD.encode(&part.payload)
		});

		let endpoint = format!("multipart/{}/parts", upload.upload_id);
		let _response = self.make_request("POST", &endpoint, Some(request_body)).await?;

		Ok(())
	}

	/// Completes a multipart upload
	pub async fn complete_multipart_upload(
		&self,
		upload: &MultipartUpload,
		part_numbers: Vec<u32>,
	) -> NeoFSResult<MultipartUploadResult> {
		let request_body = json!({
			"uploadId": upload.upload_id,
			"parts": part_numbers
		});

		let endpoint = format!("multipart/{}/complete", upload.upload_id);
		let response = self.make_request("POST", &endpoint, Some(request_body)).await?;

		let object_id = response.get("objectId").and_then(|v| v.as_str()).ok_or_else(|| {
			NeoFSError::UnexpectedResponse("Missing objectId in response".to_string())
		})?;

		Ok(MultipartUploadResult {
			object_id: ObjectId(object_id.to_string()),
			container_id: upload.container_id.clone(),
		})
	}

	/// Aborts a multipart upload
	pub async fn abort_multipart_upload(&self, upload: &MultipartUpload) -> NeoFSResult<()> {
		let endpoint = format!("multipart/{}/abort", upload.upload_id);
		let _response = self.make_request("DELETE", &endpoint, None).await?;
		Ok(())
	}
}

#[async_trait]
impl NeoFSService for NeoFSClient {
	async fn create_container(&self, container: &Container) -> NeoFSResult<ContainerId> {
		let owner_id = self.get_owner_id()?;

		let request_body = json!({
			"container": {
				"ownerId": owner_id.0,
				"basicAcl": container.basic_acl,
				"attributes": container.attributes,
				"placementPolicy": container.placement_policy
			}
		});

		let response = self.make_request("POST", "containers", Some(request_body)).await?;

		if let Some(container_id) = response.get("containerId").and_then(|v| v.as_str()) {
			Ok(ContainerId(container_id.to_string()))
		} else {
			Err(NeoFSError::UnexpectedResponse("Missing containerId in response".to_string()))
		}
	}

	async fn get_container(&self, id: &ContainerId) -> NeoFSResult<Container> {
		let endpoint = format!("containers/{}", id.0);
		let response = self.make_request("GET", &endpoint, None).await?;

		if let Some(container_data) = response.get("container") {
			let owner_id = container_data
				.get("ownerId")
				.and_then(|v| v.as_str())
				.ok_or_else(|| NeoFSError::UnexpectedResponse("Missing ownerId".to_string()))?;

			let mut container = Container::new(id.clone(), OwnerId(owner_id.to_string()));

			if let Some(basic_acl) = container_data.get("basicAcl").and_then(|v| v.as_u64()) {
				container.basic_acl = basic_acl as u32;
			}

			if let Some(attributes) = container_data.get("attributes").and_then(|v| v.as_object()) {
				for (key, value) in attributes {
					if let Some(val_str) = value.as_str() {
						container.attributes.add(key.clone(), val_str.to_string());
					}
				}
			}

			Ok(container)
		} else {
			Err(NeoFSError::UnexpectedResponse("Missing container data in response".to_string()))
		}
	}

	async fn list_containers(&self) -> NeoFSResult<Vec<ContainerId>> {
		let owner_id = self.get_owner_id()?;
		let endpoint = format!("containers?ownerId={}", owner_id.0);
		let response = self.make_request("GET", &endpoint, None).await?;

		Self::parse_container_ids_response(&response)
	}

	async fn delete_container(&self, id: &ContainerId) -> NeoFSResult<bool> {
		let endpoint = format!("containers/{}", id.0);
		let _response = self.make_request("DELETE", &endpoint, None).await?;
		Ok(true)
	}

	async fn put_object(
		&self,
		container_id: &ContainerId,
		object: &Object,
	) -> NeoFSResult<ObjectId> {
		let owner_id = self.get_owner_id()?;

		let request_body = json!({
			"object": {
				"containerId": container_id.0,
				"ownerId": owner_id.0,
				"attributes": object.attributes,
				"payload": base64::engine::general_purpose::STANDARD.encode(&object.payload)
			}
		});

		let endpoint = format!("objects/{}", container_id.0);
		let response = self.make_request("POST", &endpoint, Some(request_body)).await?;

		if let Some(object_id) = response.get("objectId").and_then(|v| v.as_str()) {
			Ok(ObjectId(object_id.to_string()))
		} else {
			Err(NeoFSError::UnexpectedResponse("Missing objectId in response".to_string()))
		}
	}

	async fn get_object(
		&self,
		container_id: &ContainerId,
		object_id: &ObjectId,
	) -> NeoFSResult<Object> {
		let endpoint = format!("objects/{}/{}", container_id.0, object_id.0);
		let response = self.make_request("GET", &endpoint, None).await?;

		if let Some(object_data) = response.get("object") {
			let owner_id = object_data
				.get("ownerId")
				.and_then(|v| v.as_str())
				.ok_or_else(|| NeoFSError::UnexpectedResponse("Missing ownerId".to_string()))?;

			let mut object = Object::new(container_id.clone(), OwnerId(owner_id.to_string()));

			if let Some(payload_b64) = object_data.get("payload").and_then(|v| v.as_str()) {
				object.payload =
					base64::engine::general_purpose::STANDARD.decode(payload_b64).map_err(|e| {
						NeoFSError::UnexpectedResponse(format!("Invalid base64 payload: {}", e))
					})?;
			}

			if let Some(attributes) = object_data.get("attributes").and_then(|v| v.as_object()) {
				for (key, value) in attributes {
					if let Some(val_str) = value.as_str() {
						object.attributes.add(key.clone(), val_str.to_string());
					}
				}
			}

			Ok(object)
		} else {
			Err(NeoFSError::UnexpectedResponse("Missing object data in response".to_string()))
		}
	}

	async fn list_objects(&self, container_id: &ContainerId) -> NeoFSResult<Vec<ObjectId>> {
		let endpoint = format!("objects/{}", container_id.0);
		let response = self.make_request("GET", &endpoint, None).await?;

		Self::parse_object_ids_response(&response)
	}

	async fn delete_object(
		&self,
		container_id: &ContainerId,
		object_id: &ObjectId,
	) -> NeoFSResult<bool> {
		let endpoint = format!("objects/{}/{}", container_id.0, object_id.0);
		let _response = self.make_request("DELETE", &endpoint, None).await?;
		Ok(true)
	}

	async fn create_bearer_token(
		&self,
		container_id: &ContainerId,
		permissions: Vec<AccessPermission>,
		expires_sec: u64,
	) -> NeoFSResult<BearerToken> {
		let _ = (container_id, permissions, expires_sec);
		Err(NeoFSError::UnsupportedOperation(
			"bearer token creation requires native NeoFS token signing; the REST gateway client does not fabricate tokens"
				.to_string(),
		))
	}

	async fn get_session_token(&self) -> NeoFSResult<SessionToken> {
		Err(NeoFSError::UnsupportedOperation(
			"session token retrieval requires a native NeoFS authentication flow; the REST gateway client does not fabricate sessions"
				.to_string(),
		))
	}

	async fn initiate_multipart_upload(
		&self,
		container_id: &ContainerId,
		object: &Object,
	) -> NeoFSResult<MultipartUpload> {
		self.init_multipart_upload(container_id, object, 1024 * 1024).await
	}

	async fn upload_part(
		&self,
		upload: &MultipartUpload,
		part_number: u32,
		data: Vec<u8>,
	) -> NeoFSResult<Part> {
		// Create the part
		let part = Part::new(part_number, data);

		// Upload the part using the internal method
		self.upload_part(upload, part.clone()).await?;

		Ok(part)
	}

	async fn complete_multipart_upload(
		&self,
		upload: &MultipartUpload,
		parts: Vec<Part>,
	) -> NeoFSResult<MultipartUploadResult> {
		// Extract part numbers from parts
		let part_numbers = parts.iter().map(|p| p.part_number).collect();
		self.complete_multipart_upload(upload, part_numbers).await
	}

	async fn abort_multipart_upload(&self, upload: &MultipartUpload) -> NeoFSResult<bool> {
		self.abort_multipart_upload(upload).await?;
		Ok(true)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_container_ids_response_rejects_missing_containers_field() {
		let response = json!({});
		let err = NeoFSClient::parse_container_ids_response(&response).unwrap_err();

		match err {
			NeoFSError::UnexpectedResponse(message) => {
				assert!(message.contains("containers"));
			},
			other => panic!("expected unexpected response error, got {other:?}"),
		}
	}

	#[test]
	fn parse_container_ids_response_rejects_missing_container_id() {
		let response = json!({ "containers": [{}] });
		let err = NeoFSClient::parse_container_ids_response(&response).unwrap_err();

		match err {
			NeoFSError::UnexpectedResponse(message) => {
				assert!(message.contains("containerId"));
			},
			other => panic!("expected unexpected response error, got {other:?}"),
		}
	}

	#[test]
	fn parse_object_ids_response_rejects_missing_objects_field() {
		let response = json!({});
		let err = NeoFSClient::parse_object_ids_response(&response).unwrap_err();

		match err {
			NeoFSError::UnexpectedResponse(message) => {
				assert!(message.contains("objects"));
			},
			other => panic!("expected unexpected response error, got {other:?}"),
		}
	}

	#[test]
	fn parse_object_ids_response_rejects_missing_object_id() {
		let response = json!({ "objects": [{}] });
		let err = NeoFSClient::parse_object_ids_response(&response).unwrap_err();

		match err {
			NeoFSError::UnexpectedResponse(message) => {
				assert!(message.contains("objectId"));
			},
			other => panic!("expected unexpected response error, got {other:?}"),
		}
	}
}
