use eframe::{egui, egui::RichText};
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio_stream::wrappers::ReceiverStream;

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
			Box::new(NeoGuiApp { state, rt })
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
}

struct NeoGuiApp {
	state: Arc<Mutex<AppState>>,
	rt: Runtime,
}

#[derive(Clone)]
struct NetworkInfo {
	endpoint: String,
	network_type: String,
	connected: bool,
	status: String,
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
		let (net, wal) = {
			let state = self.state.lock();
			(
				format!(
					"{} ({}), {}",
					state.network.endpoint,
					state.network.network_type,
					state.network.status
				),
				state.wallet_status.clone(),
			)
		};
		ui.small(format!("Network: {}", net));
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
			Tab::Wallet => ui.label("Wallet • accounts, balances, transfers (placeholder)"),
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
				self.connect_network(true);
			}
			if ui.button("Disconnect").clicked() {
				self.connect_network(false);
			}
			ui.label(status);
			if connected {
				ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "connected");
			} else {
				ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "disconnected");
			}
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

	fn connect_network(&self, connect: bool) {
		let mut state = self.state.lock();
		if connect {
			state.network.connected = true;
			state.network.status = "Connected (mock)".to_string();
			state.logs.push(format!(
				"Connected to {} [{}]",
				state.network.endpoint, state.network.network_type
			));
		} else {
			state.network.connected = false;
			state.network.status = "Disconnected".to_string();
			state.logs.push("Disconnected from network".to_string());
		}
	}
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
