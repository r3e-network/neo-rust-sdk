use eframe::{egui, egui::RichText};
use neo3::neo_builder::ScriptBuilder;
use neo3::neo_clients::{APITrait, HttpProvider, RpcClient};
use neo3::neo_protocol::{Account, AccountTrait};
use neo3::neo_types::{ContractParameter, ScriptHash, ScriptHashExtension};
use neo3::sdk::hd_wallet::{HDWallet, HDWalletBuilder};
use neo3::sdk::websocket::{SubscriptionType, WebSocketClient};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rust_decimal::prelude::{FromStr, ToPrimitive};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::{sleep, Duration};

static VERSION: Lazy<String> = Lazy::new(|| "0.5.2".to_string());
const GAS_HASH: &str = "d2a4cff31913016155e38e474a2c06d08be276cf";
const NEO_HASH: &str = "c56f33fc6ecfcd0c225c4ab356fee59390af8560";

fn main() -> eframe::Result<()> {
	let native_options = eframe::NativeOptions { follow_system_theme: true, ..Default::default() };

	eframe::run_native(
		"NeoRust GUI (Native)",
		native_options,
		Box::new(|_cc| {
			let rt = Runtime::new().expect("Failed to build tokio runtime");
			let state = Arc::new(Mutex::new(AppState::default()));
			let (tx, rx) = unbounded_channel();
			spawn_background(rx, state.clone(), rt.handle().clone());
			Box::new(NeoGuiApp { state, rt, tx, simulator_script: String::new() })
		}),
	)
}

#[derive(Copy, Clone)]
enum Tab {
	Dashboard,
	Wallet,
	HdWallet,
	Simulator,
	WebSocket,
	Analytics,
	Settings,
}

impl Default for Tab {
	fn default() -> Self {
		Tab::Dashboard
	}
}

struct AppState {
	current_tab: Tab,
	network: NetworkInfo,
	wallet_status: String,
	logs: Vec<String>,
	client: Option<Arc<RpcClient<HttpProvider>>>,
	poller_running: bool,
	last_height: Option<u32>,
	peer_count: Option<usize>,
	version: Option<String>,
	accounts: Vec<AccountInfo>,
	wif_input: String,
	hd_wallet: Option<HDWallet>,
	hd_mnemonic: String,
	hd_mnemonic_input: String,
	hd_passphrase: String,
	hd_word_count: usize,
	hd_derivation_path: String,
	hd_accounts: Vec<AccountInfo>,
	ws_url: String,
	ws_status: String,
	ws_connected: bool,
	ws_events: Vec<String>,
	ws_client: Option<WebSocketClient>,
	simulator_result: String,
	ws_subscription: String,
	transfer_to: String,
	transfer_amount: String,
}

impl Default for AppState {
	fn default() -> Self {
		Self {
			current_tab: Tab::Dashboard,
			network: NetworkInfo::default(),
			wallet_status: String::new(),
			logs: Vec::new(),
			client: None,
			poller_running: false,
			last_height: None,
			peer_count: None,
			version: None,
			accounts: Vec::new(),
			wif_input: String::new(),
			hd_wallet: None,
			hd_mnemonic: String::new(),
			hd_mnemonic_input: String::new(),
			hd_passphrase: String::new(),
			hd_word_count: 12,
			hd_derivation_path: "m/44'/888'/0'/0/0".to_string(),
			hd_accounts: Vec::new(),
			ws_url: "wss://testnet1.neo.org:7443/ws".to_string(),
			ws_status: "WebSocket idle".to_string(),
			ws_connected: false,
			ws_events: Vec::new(),
			ws_client: None,
			simulator_result: "Enter a script and run to simulate".to_string(),
			ws_subscription: "NewBlocks".to_string(),
			transfer_to: String::new(),
			transfer_amount: "1".to_string(),
		}
	}
}

struct NeoGuiApp {
	state: Arc<Mutex<AppState>>,
	#[allow(dead_code)]
	rt: Runtime,
	tx: UnboundedSender<Action>,
	simulator_script: String,
}

#[derive(Clone)]
struct NetworkInfo {
	endpoint: String,
	network_type: String,
	connected: bool,
	status: String,
}

#[derive(Clone)]
struct AccountInfo {
	address: String,
	scripthash: String,
	wif: Option<String>,
	unclaimed_gas: Option<String>,
	neo_balance: Option<String>,
	gas_balance: Option<String>,
}

impl Default for NetworkInfo {
	fn default() -> Self {
		Self {
			endpoint: "https://testnet1.neo.org:443".to_string(),
			network_type: "testnet".to_string(),
			connected: false,
			status: "Not connected".to_string(),
		}
	}
}

impl NeoGuiApp {
	fn render_sidebar(&mut self, ui: &mut egui::Ui) {
		ui.heading(RichText::new("NeoRust SDK").strong());
		ui.label(format!("Native GUI · v{}", *VERSION));
		ui.separator();
		ui.label("Network endpoint");
		{
			let mut state = self.state.lock();
			ui.text_edit_singleline(&mut state.network.endpoint);
			ui.horizontal(|ui| {
				ui.label("Type");
				egui::ComboBox::from_label("")
					.selected_text(state.network.network_type.clone())
					.show_ui(ui, |ui| {
						ui.selectable_value(
							&mut state.network.network_type,
							"mainnet".to_string(),
							"mainnet",
						);
						ui.selectable_value(
							&mut state.network.network_type,
							"testnet".to_string(),
							"testnet",
						);
						ui.selectable_value(
							&mut state.network.network_type,
							"custom".to_string(),
							"custom",
						);
					});
			});
		}

		self.tab_button(ui, Tab::Dashboard, "Dashboard");
		self.tab_button(ui, Tab::Wallet, "Wallet");
		self.tab_button(ui, Tab::HdWallet, "HD Wallet");
		self.tab_button(ui, Tab::Simulator, "Simulator");
		self.tab_button(ui, Tab::WebSocket, "WebSocket Monitor");
		self.tab_button(ui, Tab::Analytics, "Analytics");
		self.tab_button(ui, Tab::Settings, "Settings");

		ui.separator();
		ui.label("Status");
		let (net, wal, ver, peers) = {
			let state = self.state.lock();
			(
				format!(
					"{} ({}), {}",
					state.network.endpoint, state.network.network_type, state.network.status
				),
				state.wallet_status.clone(),
				state.version.clone().unwrap_or_else(|| "unknown".to_string()),
				state.peer_count,
			)
		};
		ui.small(format!("Network: {}", net));
		ui.small(format!("Version: {}", ver));
		if let Some(p) = peers {
			ui.small(format!("Peers: {}", p));
		}
		let ws_line = {
			let s = self.state.lock();
			format!("WebSocket: {}", s.ws_status)
		};
		ui.small(ws_line);
		ui.small(format!("Wallet: {}", wal));
	}

	fn tab_button(&mut self, ui: &mut egui::Ui, tab: Tab, label: &str) {
		let selected = {
			let state = self.state.lock();
			matches!(state.current_tab, t if t as u8 == tab as u8)
		};
		if ui.selectable_label(selected, label).clicked() {
			self.state.lock().current_tab = tab;
		}
	}

	fn render_content(&mut self, ui: &mut egui::Ui) {
		let tab = self.state.lock().current_tab;
		match tab {
			Tab::Dashboard => self.render_dashboard(ui),
			Tab::Wallet => self.render_wallet(ui),
			Tab::HdWallet => {
				self.render_hd_wallet(ui);
			},
			Tab::Simulator => {
				self.render_simulator(ui);
			},
			Tab::WebSocket => {
				self.render_websocket(ui);
			},
			Tab::Analytics => {
				ui.label("Analytics • charts and trends (placeholder)");
			},
			Tab::Settings => {
				ui.label("Settings • network, theme, language (placeholder)");
			},
		}
	}

	fn render_dashboard(&mut self, ui: &mut egui::Ui) {
		ui.heading("Dashboard");
		ui.label("Connect to a Neo N3 node to get status and balances.");
		ui.separator();

		let (endpoint, network_type, connected, status) = {
			let state = self.state.lock();
			(
				state.network.endpoint.clone(),
				state.network.network_type.clone(),
				state.network.connected,
				state.network.status.clone(),
			)
		};

		ui.horizontal(|ui| {
			ui.label("Endpoint:");
			ui.monospace(&endpoint);
		});
		ui.horizontal(|ui| {
			ui.label("Network:");
			ui.monospace(&network_type);
		});

		ui.add_space(8.0);
		let connect_endpoint = endpoint.clone();
		let connect_network_type = network_type.clone();
		ui.horizontal(|ui| {
			if ui.button("Connect").clicked() {
				self.queue_action(Action::Connect {
					endpoint: connect_endpoint.clone(),
					network_type: connect_network_type.clone(),
				});
			}
			if ui.button("Disconnect").clicked() {
				self.queue_action(Action::Disconnect);
			}
			ui.label(status);
			if connected {
				ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "connected");
			} else {
				ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "disconnected");
			}
		});

		ui.separator();
		ui.horizontal(|ui| {
			if ui.button("Refresh balances").clicked() {
				self.queue_action(Action::RefreshBalances);
			}
			if ui.button("Fetch NEP-17 balances").clicked() {
				self.queue_action(Action::FetchBalances);
			}
			ui.label("Refreshes unclaimed GAS for local accounts (requires connection).");
		});

		ui.separator();
		ui.label("Activity");
		egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
			let logs = self.state.lock().logs.clone();
			for entry in logs.iter().rev() {
				ui.label(entry);
			}
		});
	}

	fn queue_action(&self, action: Action) {
		let _ = self.tx.send(action);
	}

	fn render_wallet(&mut self, ui: &mut egui::Ui) {
		ui.heading("Wallet");
		ui.label("Local account management (offline key generation).");
		ui.add_space(6.0);

		ui.horizontal(|ui| {
			ui.label("Import WIF:");
			let mut state = self.state.lock();
			ui.text_edit_singleline(&mut state.wif_input);
			if ui.button("Import").clicked() {
				let wif = state.wif_input.trim().to_string();
				drop(state);
				if wif.is_empty() {
					self.state.lock().logs.push("WIF import: input is empty".to_string());
				} else {
					match Account::from_wif(&wif) {
						Ok(acc) => {
							let info = AccountInfo {
								address: acc.get_address(),
								scripthash: format!("{}", acc.get_script_hash()),
								wif: Some(wif.clone()),
								unclaimed_gas: None,
								neo_balance: None,
								gas_balance: None,
							};
							let mut s = self.state.lock();
							s.accounts.push(info.clone());
							s.logs.push(format!("Imported account {}", info.address));
							s.wif_input.clear();
						},
						Err(e) => {
							self.state.lock().logs.push(format!("WIF import failed: {}", e));
						},
					}
				}
			}
		});

		ui.add_space(4.0);
		if ui.button("Create new account").clicked() {
			match Account::create() {
				Ok(acc) => {
					let info = AccountInfo {
						address: acc.get_address(),
						scripthash: format!("{}", acc.get_script_hash()),
						wif: acc.key_pair().as_ref().map(|kp| kp.export_as_wif()),
						unclaimed_gas: None,
						neo_balance: None,
						gas_balance: None,
					};
					let mut s = self.state.lock();
					s.accounts.push(info.clone());
					s.logs.push(format!("Created account {}", info.address));
				},
				Err(e) => {
					let mut s = self.state.lock();
					s.logs.push(format!("Account creation failed: {}", e));
				},
			}
		}

		ui.separator();
		ui.label("Accounts");
		egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
			let accounts = self.state.lock().accounts.clone();
			if accounts.is_empty() {
				ui.label("No accounts created yet.");
			} else {
				for acc in accounts {
					ui.group(|ui| {
						ui.label(RichText::new(&acc.address).strong());
						ui.monospace(acc.scripthash.clone());
						if let Some(wif) = &acc.wif {
							ui.label("WIF:");
							ui.monospace(wif);
						}
						if let Some(gas) = &acc.unclaimed_gas {
							ui.label(format!("Unclaimed GAS: {}", gas));
						}
						if let Some(neo) = &acc.neo_balance {
							ui.label(format!("NEO: {}", neo));
						}
						if let Some(gas) = &acc.gas_balance {
							ui.label(format!("GAS: {}", gas));
						}
					});
				}
			}
		});

		ui.separator();
		ui.label("Transfer (NEP-17) — demo flow");
		{
			let mut state = self.state.lock();
			ui.horizontal(|ui| {
				ui.label("Recipient:");
				ui.text_edit_singleline(&mut state.transfer_to);
			});
			ui.horizontal(|ui| {
				ui.label("Amount:");
				ui.text_edit_singleline(&mut state.transfer_amount);
			});
			if ui.button("Draft").clicked() {
				self.queue_action(Action::DraftTransfer {
					to: state.transfer_to.clone(),
					amount: state.transfer_amount.clone(),
				});
			}
		}
		ui.small("GAS transfers: draft validates addresses/amounts; signing/send coming soon.");
	}

	fn render_hd_wallet(&mut self, ui: &mut egui::Ui) {
		ui.heading("HD Wallet");
		ui.label("Generate or import a mnemonic, then derive accounts with BIP-44 paths.");
		ui.add_space(6.0);

		// Generation/import controls
		let mut generate = false;
		let mut import = false;
		{
			let mut state = self.state.lock();
			ui.horizontal(|ui| {
				ui.label("Word count:");
				egui::ComboBox::from_id_source("hd_word_count")
					.selected_text(state.hd_word_count.to_string())
					.show_ui(ui, |ui| {
						ui.selectable_value(&mut state.hd_word_count, 12, "12");
						ui.selectable_value(&mut state.hd_word_count, 24, "24");
					});
				ui.label("Passphrase (optional):");
				ui.text_edit_singleline(&mut state.hd_passphrase);
			});

			if ui.button("Generate new mnemonic").clicked() {
				generate = true;
			}

			ui.separator();
			ui.label("Import existing mnemonic:");
			ui.text_edit_multiline(&mut state.hd_mnemonic_input);
			if ui.button("Import").clicked() {
				import = true;
			}
		}

		if generate {
			let (word_count, passphrase) = {
				let s = self.state.lock();
				(s.hd_word_count, s.hd_passphrase.clone())
			};
			let passphrase_opt =
				if passphrase.is_empty() { None } else { Some(passphrase.as_str()) };
			match HDWallet::generate(word_count, passphrase_opt) {
				Ok(wallet) => {
					let phrase = wallet.mnemonic_phrase().to_string();
					let mut s = self.state.lock();
					s.hd_wallet = Some(wallet);
					s.hd_mnemonic = phrase.clone();
					s.hd_accounts.clear();
					s.logs.push(format!("Generated HD wallet ({} words)", word_count));
				},
				Err(e) => {
					self.state.lock().logs.push(format!("HD wallet generation failed: {}", e));
				},
			}
		}

		if import {
			let (mnemonic, passphrase) = {
				let s = self.state.lock();
				(s.hd_mnemonic_input.trim().to_string(), s.hd_passphrase.clone())
			};
			if mnemonic.is_empty() {
				self.state.lock().logs.push("HD import failed: mnemonic is empty".to_string());
			} else {
				let mut builder = HDWalletBuilder::new().mnemonic(mnemonic.clone());
				if !passphrase.is_empty() {
					builder = builder.passphrase(passphrase.clone());
				}
				match builder.build() {
					Ok(wallet) => {
						let phrase = wallet.mnemonic_phrase().to_string();
						let mut s = self.state.lock();
						s.hd_wallet = Some(wallet);
						s.hd_mnemonic = phrase.clone();
						s.hd_accounts.clear();
						s.logs.push("Imported HD wallet mnemonic".to_string());
					},
					Err(e) => {
						self.state.lock().logs.push(format!("HD wallet import failed: {}", e));
					},
				}
			}
		}

		ui.separator();
		{
			let mut state = self.state.lock();
			ui.label("Active mnemonic:");
			if state.hd_mnemonic.is_empty() {
				ui.label("None loaded.");
			} else {
				ui.code(state.hd_mnemonic.clone());
			}

			ui.add_space(6.0);
			ui.label("Derivation path (e.g., m/44'/888'/0'/0/0):");
			ui.text_edit_singleline(&mut state.hd_derivation_path);
			if ui.button("Derive account").clicked() {
				let path = state.hd_derivation_path.clone();
				if let Some(wallet) = state.hd_wallet.as_mut() {
					match wallet.derive_account(&path) {
						Ok(acc) => {
							let info = AccountInfo {
								address: acc.get_address(),
								scripthash: format!("{}", acc.get_script_hash()),
								wif: acc.key_pair().as_ref().map(|kp| kp.export_as_wif()),
								unclaimed_gas: None,
								neo_balance: None,
								gas_balance: None,
							};
							// Add to HD list
							if !state.hd_accounts.iter().any(|a| a.address == info.address) {
								state.hd_accounts.push(info.clone());
							}
							// Deduplicate in main wallet list
							if !state.accounts.iter().any(|a| a.address == info.address) {
								state.accounts.push(info.clone());
							}
							state
								.logs
								.push(format!("Derived account {} from {}", info.address, path));
						},
						Err(e) => {
							state.logs.push(format!("Derivation failed: {}", e));
						},
					}
				} else {
					state.logs.push("No HD wallet loaded; generate/import first.".to_string());
				}
			}
		}

		ui.separator();
		ui.label("Derived accounts");
		egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
			let accounts = self.state.lock().hd_accounts.clone();
			if accounts.is_empty() {
				ui.label("No derived accounts yet.");
			} else {
				for acc in accounts {
					ui.group(|ui| {
						ui.label(RichText::new(&acc.address).strong());
						ui.monospace(acc.scripthash.clone());
						if let Some(wif) = &acc.wif {
							ui.label("WIF:");
							ui.monospace(wif);
						}
						if let Some(gas) = &acc.unclaimed_gas {
							ui.label(format!("Unclaimed GAS: {}", gas));
						}
						if let Some(neo) = &acc.neo_balance {
							ui.label(format!("NEO: {}", neo));
						}
						if let Some(gas) = &acc.gas_balance {
							ui.label(format!("GAS: {}", gas));
						}
					});
				}
			}
		});
	}

	fn render_websocket(&mut self, ui: &mut egui::Ui) {
		ui.heading("WebSocket Monitor");
		ui.label("Subscribe to real-time events (NewBlocks).");
		ui.add_space(6.0);

		let (mut url, status, connected) = {
			let s = self.state.lock();
			(s.ws_url.clone(), s.ws_status.clone(), s.ws_connected)
		};

		let mut subscription = {
			let s = self.state.lock();
			s.ws_subscription.clone()
		};

		ui.horizontal(|ui| {
			ui.label("WebSocket URL:");
			ui.text_edit_singleline(&mut url);
		});

		ui.horizontal(|ui| {
			ui.label("Subscription:");
			egui::ComboBox::from_id_source("ws_subscription")
				.selected_text(subscription.clone())
				.show_ui(ui, |ui| {
					ui.selectable_value(&mut subscription, "NewBlocks".to_string(), "NewBlocks");
					ui.selectable_value(
						&mut subscription,
						"NewTransactions".to_string(),
						"NewTransactions",
					);
					ui.selectable_value(
						&mut subscription,
						"ExecutionResults".to_string(),
						"ExecutionResults",
					);
				});
		});

		let mut should_connect = false;
		{
			let mut state = self.state.lock();
			state.ws_url = url.clone();
			state.ws_subscription = subscription.clone();
			ui.horizontal(|ui| {
				if ui.button("Connect").clicked() {
					should_connect = true;
				}
				if ui.button("Disconnect").clicked() {
					self.queue_action(Action::WsDisconnect);
				}
				ui.label(status);
				if connected {
					ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "connected");
				} else {
					ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "disconnected");
				}
			});
		}

		if should_connect {
			self.queue_action(Action::WsConnect { url, subscription });
		}

		ui.separator();
		ui.label("Events");
		egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
			let events = self.state.lock().ws_events.clone();
			if events.is_empty() {
				ui.label("No events yet.");
			} else {
				for evt in events.iter().rev() {
					ui.monospace(evt);
				}
			}
		});
	}

	fn render_simulator(&mut self, ui: &mut egui::Ui) {
		ui.heading("Transaction Simulator");
		ui.label(
			"Enter a hex-encoded script to run a dry-run simulation (uses current RPC connection).",
		);
		ui.add_space(6.0);

		ui.label("Script (hex):");
		ui.add(
			egui::TextEdit::multiline(&mut self.simulator_script)
				.desired_rows(4)
				.font(egui::TextStyle::Monospace),
		);

		let (connected, status) = {
			let s = self.state.lock();
			(s.network.connected, s.network.status.clone())
		};

		ui.horizontal(|ui| {
			if ui.button("Simulate").clicked() {
				let script_hex = self.simulator_script.trim().to_string();
				self.queue_action(Action::Simulate { script_hex });
			}
			ui.label(format!("RPC: {}", status));
			if connected {
				ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "connected");
			} else {
				ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "disconnected");
			}
		});

		ui.separator();
		let result = {
			let s = self.state.lock();
			s.simulator_result.clone()
		};
		ui.label("Result");
		ui.code(result);
	}
}

#[derive(Clone)]
enum Action {
	Connect { endpoint: String, network_type: String },
	Disconnect,
	RefreshBalances,
	FetchBalances,
	WsConnect { url: String, subscription: String },
	WsDisconnect,
	Simulate { script_hex: String },
	DraftTransfer { to: String, amount: String },
}

fn spawn_background(
	mut rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
	state: Arc<Mutex<AppState>>,
	handle: tokio::runtime::Handle,
) {
	let background_handle = handle.clone();
	let poll_handle = handle.clone();
	background_handle.spawn(async move {
		while let Some(msg) = rx.recv().await {
			match msg {
				Action::Connect { endpoint, network_type } => {
					{
						let mut s = state.lock();
						s.network.status = "Connecting...".to_string();
						s.logs.push(format!("Connecting to {} [{}]", endpoint, network_type));
					}
					let provider = match HttpProvider::new(endpoint.as_str()) {
						Ok(p) => p,
						Err(e) => {
							let mut s = state.lock();
							s.network.status = format!("Error: {}", e);
							s.logs.push(format!("Connection failed: {}", e));
							continue;
						},
					};
					let client = RpcClient::new(provider);
					// Probe the node
					let status_result = client.get_block_count().await;
					let mut s = state.lock();
					match status_result {
						Ok(height) => {
							s.network.connected = true;
							s.network.endpoint = endpoint;
							s.network.network_type = network_type;
							s.network.status = format!("Connected · height {}", height);
							s.logs.push(format!("Connected. Height: {}", height));
							s.client = Some(Arc::new(client));
							s.last_height = Some(height);
							s.peer_count = None;
							s.version = None;
						},
						Err(e) => {
							s.network.connected = false;
							s.network.status = format!("Error: {}", e);
							s.logs.push(format!("Connection failed: {}", e));
							s.client = None;
						},
					}
					// start status poller if not running
					let should_spawn = {
						let s = state.lock();
						s.network.connected && !s.poller_running
					};
					if should_spawn {
						if let Some(c) = state.lock().client.clone() {
							let state_clone = state.clone();
							let poll_handle = poll_handle.clone();
							poll_handle.spawn(status_poller(c, state_clone));
							let mut s = state.lock();
							s.poller_running = true;
						}
					}
				},
				Action::WsConnect { url, subscription } => {
					{
						let mut s = state.lock();
						s.ws_status = "Connecting...".to_string();
						s.ws_events.clear();
						s.logs.push(format!("WS connecting to {} ({})", url, subscription));
					}
					match WebSocketClient::new(&url).await {
						Ok(mut client) => {
							let connect_result = client.connect().await;
							if let Err(e) = connect_result {
								let mut s = state.lock();
								s.ws_status = format!("WS error: {}", e);
								s.logs.push(format!("WS connect failed: {}", e));
								continue;
							}

							// Subscribe to chosen type
							let sub_type = match subscription.as_str() {
								"NewTransactions" => SubscriptionType::NewTransactions,
								"ExecutionResults" => SubscriptionType::ExecutionResults,
								_ => SubscriptionType::NewBlocks,
							};
							match client.subscribe(sub_type.clone()).await {
								Ok(_) => {
									let mut s = state.lock();
									s.ws_status = format!("WS connected ({})", subscription);
									s.ws_connected = true;
									s.ws_client = Some(client);
									s.logs.push(format!("WS subscribed to {}", subscription));
								},
								Err(e) => {
									let mut s = state.lock();
									s.ws_status = format!("WS subscribe failed: {}", e);
									s.logs.push(format!("WS subscribe failed: {}", e));
									continue;
								},
							}

							// Spawn event reader
							let mut rx_opt = {
								let mut s = state.lock();
								s.ws_client.as_mut().and_then(|c| c.take_event_receiver())
							};
							if let Some(mut rx) = rx_opt.take() {
								let event_state = state.clone();
								let event_handle = handle.clone();
								event_handle.spawn(async move {
									while let Some((typ, evt)) = rx.recv().await {
										let mut s = event_state.lock();
										let line = format!("{:?}: {:?}", typ, evt);
										s.ws_events.push(line);
										if s.ws_events.len() > 200 {
											let drain = s.ws_events.len() - 200;
											s.ws_events.drain(0..drain);
										}
									}
								});
							}
						},
						Err(e) => {
							let mut s = state.lock();
							s.ws_status = format!("WS error: {}", e);
							s.logs.push(format!("WS client init failed: {}", e));
						},
					}
				},
				Action::WsDisconnect => {
					let mut client = {
						let mut s = state.lock();
						s.logs.push("WS disconnecting...".to_string());
						s.ws_status = "Disconnecting WS...".to_string();
						s.ws_client.take()
					};
					if let Some(ref mut c) = client {
						let _ = c.disconnect().await;
					}
					let mut s = state.lock();
					s.ws_connected = false;
					s.ws_status = "WebSocket disconnected".to_string();
					s.logs.push("WS disconnected".to_string());
					s.ws_subscription = "NewBlocks".to_string();
				},
				Action::Disconnect => {
					{
						let mut s = state.lock();
						s.network.status = "Disconnecting...".to_string();
						s.logs.push("Disconnecting...".to_string());
					}
					sleep(Duration::from_millis(300)).await;
					let mut s = state.lock();
					s.network.connected = false;
					s.network.status = "Disconnected".to_string();
					s.logs.push("Disconnected.".to_string());
					s.client = None;
					s.poller_running = false;
					s.last_height = None;
				},
				Action::RefreshBalances => {
					let client = { state.lock().client.clone() };
					let accounts = { state.lock().accounts.clone() };
					if let Some(client) = client {
						for acc in accounts {
							let script_hash = match acc.address.parse::<ScriptHash>() {
								Ok(hash) => hash,
								Err(e) => {
									state
										.lock()
										.logs
										.push(format!("Invalid address {}: {}", acc.address, e));
									continue;
								},
							};
							match client.get_unclaimed_gas(script_hash).await {
								Ok(gas) => {
									let mut s = state.lock();
									if let Some(existing) =
										s.accounts.iter_mut().find(|a| a.address == acc.address)
									{
										existing.unclaimed_gas = Some(gas.unclaimed.clone());
									}
									s.logs.push(format!(
										"Unclaimed GAS for {}: {}",
										acc.address, gas.unclaimed
									));
								},
								Err(e) => {
									state.lock().logs.push(format!(
										"Failed to fetch GAS for {}: {}",
										acc.address, e
									));
								},
							}
						}
					} else {
						state.lock().logs.push("Refresh failed: not connected".to_string());
					}
				},
				Action::FetchBalances => {
					let client = { state.lock().client.clone() };
					let accounts = { state.lock().accounts.clone() };
					if let Some(client) = client {
						for acc in accounts {
							let script_hash = match acc.address.parse::<ScriptHash>() {
								Ok(h) => h,
								Err(e) => {
									state
										.lock()
										.logs
										.push(format!("Invalid address {}: {}", acc.address, e));
									continue;
								},
							};
							match client.get_nep17_balances(script_hash).await {
								Ok(balances) => {
									let mut s = state.lock();
									if let Some(existing) =
										s.accounts.iter_mut().find(|a| a.address == acc.address)
									{
										existing.neo_balance = None;
										existing.gas_balance = None;
										for bal in &balances.balances {
											let hash = bal.asset_hash.to_string().to_lowercase();
											let normalized = hash.trim_start_matches("0x");
											if normalized == GAS_HASH {
												existing.gas_balance = Some(bal.amount.clone());
											} else if normalized == NEO_HASH {
												existing.neo_balance = Some(bal.amount.clone());
											}
										}
									}
									s.logs.push(format!(
										"Fetched NEP-17 balances for {}",
										acc.address
									));
								},
								Err(e) => {
									state.lock().logs.push(format!(
										"Balance fetch failed for {}: {}",
										acc.address, e
									));
								},
							}
						}
					} else {
						state.lock().logs.push("Balance fetch failed: not connected".to_string());
					}
				},
				Action::Simulate { script_hex } => {
					let client = { state.lock().client.clone() };
					if client.is_none() {
						let mut s = state.lock();
						s.logs.push("Simulation failed: not connected".to_string());
						continue;
					}
					let script_bytes = match hex::decode(script_hex.trim_start_matches("0x")) {
						Ok(b) => b,
						Err(e) => {
							let mut s = state.lock();
							s.logs.push(format!("Simulation failed: invalid hex - {}", e));
							continue;
						},
					};
					let response = client
						.as_ref()
						.unwrap()
						.invoke_script(hex::encode(script_bytes), vec![])
						.await;
					let mut s = state.lock();
					match response {
						Ok(result) => {
							s.logs.push("Simulation success".to_string());
							let summary = format!(
								"State: {:?} · Gas: {} · Stack items: {}",
								result.state,
								result.gas_consumed,
								result.stack.len()
							);
							s.logs.push(summary.clone());
							s.simulator_result = summary;
						},
						Err(e) => {
							let msg = format!("Simulation error: {}", e);
							s.logs.push(msg.clone());
							s.simulator_result = msg;
						},
					}
				},
				Action::DraftTransfer { to, amount } => {
					let sender = { state.lock().accounts.get(0).cloned() };
					let sender = match sender {
						Some(acc) => acc,
						None => {
							state.lock().logs.push(
								"Transfer draft failed: create or import an account first"
									.to_string(),
							);
							continue;
						},
					};
					let value = match Decimal::from_str(&amount) {
						Ok(v) => v,
						Err(e) => {
							state.lock().logs.push(format!("Invalid amount {}: {}", amount, e));
							continue;
						},
					};
					let _to_hash = match to.parse::<ScriptHash>() {
						Ok(h) => h,
						Err(e) => {
							state.lock().logs.push(format!("Invalid recipient {}: {}", to, e));
							continue;
						},
					};

					let _from_hash = match ScriptHash::from_address(&sender.address) {
						Ok(h) => h,
						Err(e) => {
							state
								.lock()
								.logs
								.push(format!("Invalid sender address {}: {}", sender.address, e));
							continue;
						},
					};

					// Build GAS transfer script
					let _amount_i32 = value.try_into().unwrap_or(0);

					// Best-effort invoke_script estimation
					let from_hash = match ScriptHash::from_address(&sender.address) {
						Ok(h) => h,
						Err(e) => {
							state
								.lock()
								.logs
								.push(format!("Invalid sender address {}: {}", sender.address, e));
							continue;
						},
					};
					let to_hash = match to.parse::<ScriptHash>() {
						Ok(h) => h,
						Err(e) => {
							state.lock().logs.push(format!("Invalid recipient {}: {}", to, e));
							continue;
						},
					};
					let scaled = (value * Decimal::from(100_000_000u64)).to_i64().unwrap_or(0);
					let mut script_builder = ScriptBuilder::new();
					let gas_hash =
						ScriptHash::from_hex(GAS_HASH).unwrap_or_else(|_| ScriptHash::zero());
					let params = vec![
						ContractParameter::h160(&from_hash),
						ContractParameter::h160(&to_hash),
						ContractParameter::integer(scaled),
						ContractParameter::any(),
					];
					let script =
						match script_builder.contract_call(&gas_hash, "transfer", &params, None) {
							Ok(_) => script_builder.to_bytes(),
							Err(e) => {
								state
									.lock()
									.logs
									.push(format!("Failed to build transfer script: {}", e));
								continue;
							},
						};
					let client = { state.lock().client.clone() };
					if let Some(client) = client {
						match client.invoke_script(hex::encode(script), vec![]).await {
							Ok(result) => {
								let mut s = state.lock();
								s.logs.push(format!(
									"Draft transfer {} GAS -> {} | state={:?} gas={} stack={}",
									amount,
									to,
									result.state,
									result.gas_consumed,
									result.stack.len()
								));
							},
							Err(e) => {
								state.lock().logs.push(format!("Draft invoke failed: {}", e));
							},
						}
					} else {
						state
							.lock()
							.logs
							.push("Draft transfer: connect to RPC for estimation".to_string());
					}
				},
			}
		}
	});
}

async fn status_poller(client: Arc<RpcClient<HttpProvider>>, state: Arc<Mutex<AppState>>) {
	loop {
		sleep(Duration::from_secs(5)).await;
		let connected = {
			let s = state.lock();
			s.network.connected
		};
		if !connected {
			break;
		}

		match client.get_block_count().await {
			Ok(height) => {
				let mut s = state.lock();
				let changed = s.last_height.map(|h| h != height).unwrap_or(true);
				s.last_height = Some(height);
				s.network.status = format!("Connected · height {}", height);
				if changed {
					s.logs.push(format!("Height: {}", height));
				}
			},
			Err(e) => {
				let mut s = state.lock();
				s.network.status = format!("Error: {}", e);
				s.logs.push(format!("Status poll failed: {}", e));
			},
		}

		// version/peers best-effort
		match client.get_version().await {
			Ok(v) => {
				let mut s = state.lock();
				s.version = Some(v.user_agent.clone());
			},
			Err(e) => {
				let mut s = state.lock();
				s.logs.push(format!("Version fetch failed: {}", e));
			},
		}

		match client.get_peers().await {
			Ok(peers) => {
				let mut s = state.lock();
				let count = peers.connected.len() + peers.unconnected.len() + peers.bad.len();
				s.peer_count = Some(count);
			},
			Err(e) => {
				let mut s = state.lock();
				s.logs.push(format!("Peers fetch failed: {}", e));
			},
		}
	}
	let mut s = state.lock();
	s.poller_running = false;
}

impl eframe::App for NeoGuiApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.heading(RichText::new("NeoRust Native GUI").strong());
				ui.separator();
				ui.label(format!("Version {}", *VERSION));
			});
		});

		egui::SidePanel::left("sidebar")
			.resizable(false)
			.default_width(180.0)
			.show(ctx, |ui| {
				self.render_sidebar(ui);
			});

		egui::CentralPanel::default().show(ctx, |ui| {
			self.render_content(ui);
		});
	}
}
