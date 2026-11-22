#![allow(dead_code)]
use crate::{
	commands::wallet::CliState,
	errors::CliError,
	utils::config::{NeoFSEndpoint, load_config, save_config},
	utils::{print_info, print_success},
};
use clap::{Args, Subcommand};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use url::Url;

// NeoFS endpoint constants
const DEFAULT_MAINNET_ENDPOINT: &str = "https://grpc.fs.neo.org";
const DEFAULT_TESTNET_ENDPOINT: &str = "https://grpc.testnet.fs.neo.org";
const DEFAULT_MAINNET_HTTP_GATEWAY: &str = "https://http.fs.neo.org";
const DEFAULT_TESTNET_HTTP_GATEWAY: &str = "https://http.testnet.fs.neo.org";
const DEFAULT_MAINNET_REST_ENDPOINT: &str = "https://rest.fs.neo.org";
const DEFAULT_TESTNET_REST_ENDPOINT: &str = "https://rest.testnet.fs.neo.org";

use reqwest::{Client as HttpClient, Response};
use serde::{Deserialize, Serialize};

// Production-ready NeoFS client
struct NeoFSClient {
	grpc_endpoint: String,
	http_gateway: String,
	rest_endpoint: String,
	http_client: HttpClient,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerInfo {
	pub id: String,
	pub name: String,
	pub owner: String,
	pub created_at: String,
	pub basic_acl: u32,
	pub placement_policy: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ObjectInfo {
	pub id: String,
	pub container_id: String,
	pub owner: String,
	pub created_at: String,
	pub size: u64,
	pub checksum: String,
	pub content_type: String,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
	#[serde(alias = "objectId", alias = "objectID")]
	object_id: String,
	#[serde(alias = "containerId", alias = "containerID")]
	container_id: String,
}

#[derive(Debug)]
struct UploadedObjectInfo {
	pub id: String,
	pub container_id: String,
	pub size: u64,
	pub checksum: String,
	pub content_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkStatus {
	pub status: String,
	pub network: String,
	pub version: String,
	pub nodes: u32,
	pub epoch: u64,
}

impl NeoFSClient {
	fn default() -> Self {
		Self {
			grpc_endpoint: DEFAULT_MAINNET_ENDPOINT.to_string(),
			http_gateway: DEFAULT_MAINNET_HTTP_GATEWAY.to_string(),
			rest_endpoint: DEFAULT_MAINNET_REST_ENDPOINT.to_string(),
			http_client: HttpClient::new(),
		}
	}

	fn with_endpoint(endpoint: &str) -> Self {
		let is_testnet = endpoint.contains("testnet");
		let (mut grpc, mut http, mut rest) = if is_testnet {
			(
				DEFAULT_TESTNET_ENDPOINT.to_string(),
				DEFAULT_TESTNET_HTTP_GATEWAY.to_string(),
				DEFAULT_TESTNET_REST_ENDPOINT.to_string(),
			)
		} else {
			(
				endpoint.to_string(),
				DEFAULT_MAINNET_HTTP_GATEWAY.to_string(),
				DEFAULT_MAINNET_REST_ENDPOINT.to_string(),
			)
		};

		if endpoint.contains("rest.") {
			rest = endpoint.to_string();
		} else if endpoint.contains("http.") {
			http = endpoint.to_string();
		} else {
			grpc = endpoint.to_string();
		}

		Self {
			grpc_endpoint: grpc,
			http_gateway: http,
			rest_endpoint: rest,
			http_client: HttpClient::new(),
		}
	}

	async fn parse_json<T: DeserializeOwned>(
		response: Response,
		context: &str,
	) -> Result<T, CliError> {
		let status = response.status();
		if !status.is_success() {
			let body = response.text().await.unwrap_or_default();
			let detail =
				if body.trim().is_empty() { String::new() } else { format!(": {}", body.trim()) };
			return Err(CliError::Network(format!("{context}: HTTP {status}{detail}")));
		}

		response
			.json::<T>()
			.await
			.map_err(|e| CliError::Network(format!("{context}: invalid JSON response: {e}")))
	}

	async fn get_network_status(&self) -> Result<NetworkStatus, CliError> {
		let url = format!("{}/status", self.rest_endpoint);

		match self.http_client.get(&url).send().await {
			Ok(response) => Self::parse_json(response, "Failed to get NeoFS network status").await,
			Err(e) => Err(CliError::Network(format!("Connection error: {}", e))),
		}
	}

	async fn list_containers(&self) -> Result<Vec<ContainerInfo>, CliError> {
		let url = format!("{}/containers", self.rest_endpoint);

		match self.http_client.get(&url).send().await {
			Ok(response) => Self::parse_json(response, "Failed to list NeoFS containers").await,
			Err(e) => Err(CliError::Network(format!("Connection error: {}", e))),
		}
	}

	async fn get_container(&self, container_id: &str) -> Result<ContainerInfo, CliError> {
		let url = format!("{}/containers/{}", self.rest_endpoint, container_id);

		match self.http_client.get(&url).send().await {
			Ok(response) => Self::parse_json(response, "Failed to get NeoFS container").await,
			Err(e) => Err(CliError::Network(format!("Connection error: {}", e))),
		}
	}

	async fn list_objects(
		&self,
		container_id: &str,
		prefix: Option<&str>,
	) -> Result<Vec<ObjectInfo>, CliError> {
		let mut url = format!("{}/containers/{}/objects", self.rest_endpoint, container_id);
		if let Some(prefix) = prefix {
			url = format!("{}?prefix={}", url, prefix);
		}

		match self.http_client.get(&url).send().await {
			Ok(response) => Self::parse_json(response, "Failed to list NeoFS objects").await,
			Err(e) => Err(CliError::Network(format!("Connection error: {}", e))),
		}
	}

	async fn upload_object(
		&self,
		file_path: &PathBuf,
		container_id: &str,
		object_path: Option<&str>,
	) -> Result<UploadedObjectInfo, CliError> {
		// Read file content
		let file_content = match std::fs::read(file_path) {
			Ok(content) => content,
			Err(e) => return Err(CliError::FileSystem(format!("Failed to read file: {}", e))),
		};

		let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
		let upload_path = object_path.unwrap_or(&file_name);
		if upload_path.contains(['\r', '\n']) {
			return Err(CliError::InvalidInput(
				"Object upload path must not contain CR or LF characters".to_string(),
			));
		}
		let content_type = mime_guess::from_path(file_path)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		let size = file_content.len() as u64;
		let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&file_content)));
		let boundary = format!("neo-cli-{}", uuid::Uuid::new_v4());
		let header_filename = upload_path.replace('"', "%22");
		let mut body = Vec::with_capacity(file_content.len() + 512);
		body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
		body.extend_from_slice(
			format!(
				"Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
				header_filename
			)
			.as_bytes(),
		);
		body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
		body.extend_from_slice(&file_content);
		body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

		let url = format!("{}/upload/{}", self.http_gateway, container_id);

		match self
			.http_client
			.post(&url)
			.header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
			.body(body)
			.send()
			.await
		{
			Ok(response) => {
				let upload: UploadResponse =
					Self::parse_json(response, "Failed to upload NeoFS object").await?;
				Ok(UploadedObjectInfo {
					id: upload.object_id,
					container_id: upload.container_id,
					size,
					checksum,
					content_type,
				})
			},
			Err(e) => Err(CliError::Network(format!("Upload error: {}", e))),
		}
	}

	async fn download_object(
		&self,
		container_id: &str,
		object_id: &str,
		output_path: Option<&PathBuf>,
	) -> Result<(), CliError> {
		let url = format!("{}/get/{}/{}", self.http_gateway, container_id, object_id);

		match self.http_client.get(&url).send().await {
			Ok(response) => {
				if response.status().is_success() {
					let content = match response.bytes().await {
						Ok(bytes) => bytes,
						Err(e) => {
							return Err(CliError::Network(format!(
								"Failed to read response: {}",
								e
							)));
						},
					};

					let file_path = match output_path {
						Some(path) => path.clone(),
						None => PathBuf::from(format!("downloaded_{}", object_id)),
					};

					match std::fs::write(&file_path, content) {
						Ok(_) => {
							print_success(&format!(
								"Object downloaded to: {}",
								file_path.display()
							));
							Ok(())
						},
						Err(e) => Err(CliError::FileSystem(format!("Failed to write file: {}", e))),
					}
				} else {
					Err(CliError::Network(format!(
						"Failed to download object: HTTP {}",
						response.status()
					)))
				}
			},
			Err(e) => Err(CliError::Network(format!("Download error: {}", e))),
		}
	}
}

/// NeoFS Commands
#[derive(Args, Debug)]
pub struct NeoFSArgs {
	/// NeoFS endpoint URL
	#[arg(short, long)]
	pub endpoint: Option<String>,

	#[command(subcommand)]
	pub command: NeoFSCommands,
}

/// NeoFS Command variants
#[derive(Subcommand, Debug)]
pub enum NeoFSCommands {
	/// Container management commands
	Container {
		#[command(subcommand)]
		command: ContainerCommands,
	},

	/// Object management commands
	Object {
		#[command(subcommand)]
		command: ObjectCommands,
	},

	/// ACL management commands
	Acl {
		#[command(subcommand)]
		command: AclCommands,
	},

	/// Configuration and endpoint management
	Config {
		#[command(subcommand)]
		command: ConfigCommands,
	},

	/// Show NeoFS network status
	Status,
}

/// Container management commands
#[derive(Subcommand, Debug)]
pub enum ContainerCommands {
	/// Create a new container
	Create {
		/// Container name
		#[arg(short, long)]
		name: String,

		/// Basic ACL setting (public, private, etc.)
		#[arg(short, long)]
		acl: Option<String>,

		/// Additional container options in JSON format
		#[arg(short, long)]
		options: Option<String>,
	},

	/// List all containers
	List,

	/// Get container info
	Get {
		/// Container ID or name
		#[arg(short, long)]
		id: String,
	},

	/// Delete a container
	Delete {
		/// Container ID or name
		#[arg(short, long)]
		id: String,

		/// Force deletion without confirmation
		#[arg(short, long)]
		force: bool,
	},
}

/// Object management commands
#[derive(Subcommand, Debug)]
pub enum ObjectCommands {
	/// Upload an object to NeoFS
	Put {
		/// Path to local file
		#[arg(short, long)]
		file: PathBuf,

		/// Container ID or name
		#[arg(short, long)]
		container: String,

		/// Path within container
		#[arg(short, long)]
		path: Option<String>,
	},

	/// Download an object from NeoFS
	Get {
		/// Container ID or name
		#[arg(short, long)]
		container: String,

		/// Object ID or path
		#[arg(short = 'i', long)]
		object: String,

		/// Path to save file locally
		#[arg(short, long)]
		output: Option<PathBuf>,
	},

	/// List objects in a container
	List {
		/// Container ID or name
		#[arg(short, long)]
		container: String,

		/// Path prefix for filtering
		#[arg(short, long)]
		prefix: Option<String>,
	},

	/// Delete an object
	Delete {
		/// Container ID or name
		#[arg(short, long)]
		container: String,

		/// Object ID or path
		#[arg(short, long)]
		object: String,

		/// Force deletion without confirmation
		#[arg(short, long)]
		force: bool,
	},
}

/// ACL management commands
#[derive(Subcommand, Debug)]
pub enum AclCommands {
	/// Get ACL for a container
	Get {
		/// Container ID or name
		#[arg(short, long)]
		container: String,
	},

	/// Set ACL for a container
	Set {
		/// Container ID or name
		#[arg(short, long)]
		container: String,

		/// ACL rules in JSON format
		#[arg(short, long)]
		rules: String,
	},
}

/// Configuration commands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
	/// Set the default endpoint
	SetEndpoint {
		/// NeoFS endpoint URL
		#[arg(short, long)]
		url: String,

		/// Environment (mainnet, testnet)
		#[arg(short, long)]
		env: Option<String>,
	},

	/// Get current configuration
	Get,
}

/// Handle NeoFS commands
pub async fn handle_neofs_command(args: NeoFSArgs, _state: &mut CliState) -> Result<(), CliError> {
	let client = match args.endpoint {
		Some(endpoint) => NeoFSClient::with_endpoint(&endpoint),
		None => configured_neofs_client()?,
	};

	match args.command {
		NeoFSCommands::Container { command } => handle_container_command(command, &client).await,
		NeoFSCommands::Object { command } => handle_object_command(command, &client).await,
		NeoFSCommands::Acl { command } => handle_acl_command(command, &client).await,
		NeoFSCommands::Config { command } => handle_config_command(command).await,
		NeoFSCommands::Status => handle_status_command(&client).await,
	}
}

fn configured_neofs_client() -> Result<NeoFSClient, CliError> {
	let config = load_config()?;
	let Some(default_name) = config.neofs.default_endpoint else {
		return Ok(NeoFSClient::default());
	};

	let Some(endpoint) =
		config.neofs.endpoints.iter().find(|endpoint| endpoint.name == default_name)
	else {
		return Err(CliError::Config(format!(
			"Default NeoFS endpoint '{}' is not present in the configured endpoint list",
			default_name
		)));
	};

	Ok(NeoFSClient::with_endpoint(&endpoint.url))
}

fn unsupported_signed_neofs_operation(operation: &str) -> CliError {
	CliError::InvalidOperation(format!(
		"{operation} requires NeoFS native signed API support. The built-in HTTP gateway client \
		 supports object upload and download only, so the CLI refuses to report success for this \
		 operation without a signed NeoFS backend."
	))
}

/// Handle container commands
async fn handle_container_command(
	command: ContainerCommands,
	client: &NeoFSClient,
) -> Result<(), CliError> {
	match command {
		ContainerCommands::Create { name, acl: _, options: _ } => {
			if name.is_empty() {
				return Err(CliError::InvalidInput("Container name cannot be empty".to_string()));
			}
			Err(unsupported_signed_neofs_operation("Container creation"))
		},
		ContainerCommands::List => {
			print_info("Retrieving containers from NeoFS network...");
			match client.list_containers().await {
				Ok(containers) => {
					if containers.is_empty() {
						println!("No containers found.");
					} else {
						println!(
							"{:<32} | {:<20} | {:<20} | {}",
							"Container ID", "Name", "Owner", "Created"
						);
						println!("{}", "-".repeat(100));
						for container in containers {
							println!(
								"{:<32} | {:<20} | {:<20} | {}",
								container.id,
								container.name,
								&container.owner[..20.min(container.owner.len())],
								container.created_at
							);
						}
					}
					Ok(())
				},
				Err(e) => {
					eprintln!("Failed to list containers: {e}");
					Err(e)
				},
			}
		},
		ContainerCommands::Get { id } => {
			print_info(&format!("Retrieving container {} from NeoFS network...", id));
			match client.get_container(&id).await {
				Ok(container) => {
					println!("Container Details:");
					println!("  ID: {}", container.id);
					println!("  Name: {}", container.name);
					println!("  Owner: {}", container.owner);
					println!("  Created: {}", container.created_at);
					println!("  Basic ACL: 0x{:08X}", container.basic_acl);
					println!("  Placement Policy: {}", container.placement_policy);
					Ok(())
				},
				Err(e) => {
					eprintln!("Failed to get container: {e}");
					Err(e)
				},
			}
		},
		ContainerCommands::Delete { id, force } => {
			let _ = (id, force);
			Err(unsupported_signed_neofs_operation("Container deletion"))
		},
	}
}

/// Handle object commands
async fn handle_object_command(
	command: ObjectCommands,
	client: &NeoFSClient,
) -> Result<(), CliError> {
	match command {
		ObjectCommands::Put { file, container, path } => {
			if !file.exists() {
				return Err(CliError::FileSystem(format!("File not found: {}", file.display())));
			}

			let default_filename =
				file.file_name().unwrap_or_default().to_string_lossy().to_string();
			let path_str = path.as_deref().unwrap_or(&default_filename);
			print_info(&format!(
				"Uploading file {} to container {} at path {}...",
				file.display(),
				container,
				path_str
			));

			match client.upload_object(&file, &container, path.as_deref()).await {
				Ok(object_info) => {
					print_success(&format!("Object uploaded successfully!"));
					println!("  Object ID: {}", object_info.id);
					println!("  Container ID: {}", object_info.container_id);
					println!("  Size: {} bytes", object_info.size);
					println!("  Checksum: {}", object_info.checksum);
					println!("  Content Type: {}", object_info.content_type);
					Ok(())
				},
				Err(e) => {
					eprintln!("Failed to upload object: {e}");
					Err(e)
				},
			}
		},
		ObjectCommands::Get { container, object, output } => {
			let output_str = match &output {
				Some(path) => path.display().to_string(),
				None => format!("downloaded_{}", object),
			};
			print_info(&format!(
				"Downloading object {} from container {} to {}...",
				object, container, output_str
			));

			match client.download_object(&container, &object, output.as_ref()).await {
				Ok(_) => {
					print_success("Object downloaded successfully!");
					Ok(())
				},
				Err(e) => {
					eprintln!("Failed to download object: {e}");
					Err(e)
				},
			}
		},
		ObjectCommands::List { container, prefix } => {
			let prefix_str = prefix.as_deref().unwrap_or("");
			print_info(&format!(
				"Listing objects in container {} with prefix '{}'...",
				container, prefix_str
			));

			match client.list_objects(&container, prefix.as_deref()).await {
				Ok(objects) => {
					if objects.is_empty() {
						println!("No objects found in container.");
					} else {
						println!(
							"{:<32} | {:<20} | {:<10} | {}",
							"Object ID", "Content Type", "Size", "Created"
						);
						println!("{}", "-".repeat(80));
						for object in objects {
							println!(
								"{:<32} | {:<20} | {:<10} | {}",
								object.id,
								object.content_type,
								format!("{} B", object.size),
								object.created_at
							);
						}
					}
					Ok(())
				},
				Err(e) => {
					eprintln!("Failed to list objects: {e}");
					Err(e)
				},
			}
		},
		ObjectCommands::Delete { container, object, force } => {
			let _ = (container, object, force);
			Err(unsupported_signed_neofs_operation("Object deletion"))
		},
	}
}

/// Handle ACL commands
async fn handle_acl_command(command: AclCommands, client: &NeoFSClient) -> Result<(), CliError> {
	match command {
		AclCommands::Get { container } => {
			let _ = (container, client);
			Err(unsupported_signed_neofs_operation("ACL retrieval"))
		},
		AclCommands::Set { container, rules } => {
			let _ = (container, rules, client);
			Err(unsupported_signed_neofs_operation("ACL update"))
		},
	}
}

/// Handle configuration commands
async fn handle_config_command(command: ConfigCommands) -> Result<(), CliError> {
	match command {
		ConfigCommands::SetEndpoint { url, env } => {
			let parsed = Url::parse(&url)
				.map_err(|e| CliError::InvalidInput(format!("Invalid NeoFS endpoint URL: {e}")))?;
			let scheme = parsed.scheme();
			if scheme != "http" && scheme != "https" {
				return Err(CliError::InvalidInput(
					"NeoFS endpoint URL must use http or https".to_string(),
				));
			}

			let network = env.unwrap_or_else(|| {
				if url.contains("testnet") { "testnet".to_string() } else { "mainnet".to_string() }
			});
			let endpoint_type = if url.contains("rest.") {
				"rest"
			} else if url.contains("http.") {
				"http"
			} else {
				"grpc"
			};
			let name = format!("{}-{}", network, endpoint_type);

			let mut config = load_config()?;
			let endpoint = NeoFSEndpoint {
				name: name.clone(),
				url: url.clone(),
				network,
				endpoint_type: endpoint_type.to_string(),
			};
			if let Some(existing) =
				config.neofs.endpoints.iter_mut().find(|existing| existing.name == name)
			{
				*existing = endpoint;
			} else {
				config.neofs.endpoints.push(endpoint);
			}
			config.neofs.default_endpoint = Some(name.clone());
			save_config(&config)?;

			print_success(&format!("Default NeoFS endpoint set to '{}' ({})", name, url));
			Ok(())
		},
		ConfigCommands::Get => {
			let config = load_config()?;
			print_info("Current NeoFS configuration:");
			println!(
				"Default Endpoint: {}",
				config.neofs.default_endpoint.as_deref().unwrap_or("<none>")
			);
			for endpoint in config.neofs.endpoints {
				println!(
					"- {} [{} {}]: {}",
					endpoint.name, endpoint.network, endpoint.endpoint_type, endpoint.url
				);
			}
			Ok(())
		},
	}
}

/// Handle status command
async fn handle_status_command(client: &NeoFSClient) -> Result<(), CliError> {
	print_info(&format!("Checking NeoFS status on endpoint: {}", client.grpc_endpoint));
	let status = client.get_network_status().await?;
	println!("Status: {}", status.status);
	println!("Network: {}", status.network);
	println!("Version: {}", status.version);
	println!("Nodes: {}", status.nodes);
	println!("Epoch: {}", status.epoch);
	Ok(())
}
