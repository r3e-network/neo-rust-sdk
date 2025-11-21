use eframe::{egui, egui::RichText};
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::{sleep, Duration};
use neo3::neo_clients::{HttpProvider, RpcClient};
use neo3::neo_protocol::{Account, AccountTrait};

static VERSION: Lazy<String> = Lazy::new(|| "0.5.1".to_string());

fn main() -> eframe::Result<()> {
	let native_options = eframe::NativeOptions {
		follow_system_theme: true,
		..Default::default()
	};

	eframe::run_native(
		"NeoRust GUI (Native)",
		native_options,
		Box::new(|_cc| {
			let rt = Runtime::new().expect("Failed to build tokio runtime");
			let state = Arc::new(Mutex::new(AppState::default()));
			let (tx, rx) = unbounded_channel();
			spawn_background(rx, state.clone(), rt.handle().clone());
			Box::new(NeoGuiApp { state, rt, tx })
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

#[derive(Default)]
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
}

struct NeoGuiApp {
	state: Arc<Mutex<AppState>>,
	rt: Runtime,
	tx: UnboundedSender<Action>,
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
						ui.selectable_value(&mut state.network.network_type, "mainnet".to_string(), "mainnet");
						ui.selectable_value(&mut state.network.network_type, "testnet".to_string(), "testnet");
						ui.selectable_value(&mut state.network.network_type, "custom".to_string(), "custom");
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
					state.network.endpoint,
					state.network.network_type,
					state.network.status
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
			Tab::HdWallet => ui.label("HD Wallet • derive and manage accounts (placeholder)"),
			Tab::Simulator => ui.label("Transaction Simulator • scripts and fee estimates (placeholder)"),
			Tab::WebSocket => ui.label("WebSocket Monitor • real-time events (placeholder)"),
			Tab::Analytics => ui.label("Analytics • charts and trends (placeholder)"),
			Tab::Settings => ui.label("Settings • network, theme, language (placeholder)"),
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
			ui.monospace(endpoint);
		});
		ui.horizontal(|ui| {
			ui.label("Network:");
			ui.monospace(network_type);
		});

		ui.add_space(8.0);
		ui.horizontal(|ui| {
			if ui.button("Connect").clicked() {
				self.queue_action(Action::Connect {
					endpoint,
					network_type,
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
							};
							let mut s = self.state.lock();
							s.accounts.push(info.clone());
							s.logs.push(format!("Imported account {}", info.address));
							s.wif_input.clear();
						},
						Err(e) => {
							self.state
								.lock()
								.logs
								.push(format!("WIF import failed: {}", e));
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
						wif: acc.key_pair().and_then(|kp| kp.private_key.to_wif().ok()),
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
		ui.label("Note: balances and transfers will be wired to the SDK in upcoming iterations.");
	}
}

#[derive(Clone)]
enum Action {
	Connect { endpoint: String, network_type: String },
	Disconnect,
	RefreshBalances,
	FetchBalances,
}

fn spawn_background(
	mut rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
	state: Arc<Mutex<AppState>>,
	handle: tokio::runtime::Handle,
) {
	handle.spawn(async move {
		while let Some(msg) = rx.recv().await {
			match msg {
				Action::Connect { endpoint, network_type } => {
					{
						let mut s = state.lock();
						s.network.status = "Connecting...".to_string();
						s.logs.push(format!("Connecting to {} [{}]", endpoint, network_type));
					}
					let provider = match HttpProvider::new(&endpoint) {
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
							handle.spawn(status_poller(c, state_clone));
							let mut s = state.lock();
							s.poller_running = true;
						}
					}
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
						for mut acc in accounts {
							match client.get_unclaimed_gas(acc.address.clone().into()).await {
								Ok(gas) => {
									let mut s = state.lock();
									if let Some(existing) = s
										.accounts
										.iter_mut()
										.find(|a| a.address == acc.address)
									{
										existing.unclaimed_gas = Some(gas.unclaimed.clone());
									}
									s.logs.push(format!(
										"Unclaimed GAS for {}: {}",
										acc.address, gas.unclaimed
									));
								},
								Err(e) => {
									state
										.lock()
										.logs
										.push(format!("Failed to fetch GAS for {}: {}", acc.address, e));
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
							let script_hash = match acc.address.parse::<neo3::neo_types::ScriptHash>() {
								Ok(h) => h,
								Err(e) => {
									state
										.lock()
										.logs
										.push(format!("Invalid address {}: {}", acc.address, e));
									continue;
								},
							};
							match client.get_nep17_balances(script_hash.0).await {
								Ok(balances) => {
									let mut s = state.lock();
									if let Some(existing) = s.accounts.iter_mut().find(|a| a.address == acc.address) {
										for bal in &balances.balance {
											if bal.asset_hash.to_string().to_lowercase().contains("neo") {
												existing.neo_balance = Some(bal.amount.clone());
											}
											if bal.asset_hash.to_string().to_lowercase().contains("gas") {
												existing.gas_balance = Some(bal.amount.clone());
											}
										}
									}
									s.logs.push(format!("Fetched NEP-17 balances for {}", acc.address));
								},
								Err(e) => {
									state
										.lock()
										.logs
										.push(format!("Balance fetch failed for {}: {}", acc.address, e));
								},
							}
						}
					} else {
						state.lock().logs.push("Balance fetch failed: not connected".to_string());
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
				let count = peers.connected.len()
					+ peers.unconnected.len()
					+ peers.bad.len();
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
