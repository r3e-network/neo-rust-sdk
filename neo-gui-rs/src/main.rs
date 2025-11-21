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
	network_status: String,
	wallet_status: String,
}

struct NeoGuiApp {
	state: Arc<Mutex<AppState>>,
	rt: Runtime,
}

impl NeoGuiApp {
	fn render_sidebar(&mut self, ui: &mut egui::Ui) {
		ui.heading(RichText::new("NeoRust SDK").strong());
		ui.label(format!("Native GUI · v{}", *VERSION));
		ui.separator();
		ui.label("Network endpoint");
		ui.text_edit_singleline(&mut self.state.lock().network_status);

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
			(state.network_status.clone(), state.wallet_status.clone())
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
			Tab::Dashboard => ui.label("Dashboard • metrics and quick actions (placeholder)"),
			Tab::Wallet => ui.label("Wallet • accounts, balances, transfers (placeholder)"),
			Tab::HdWallet => ui.label("HD Wallet • derive and manage accounts (placeholder)"),
			Tab::Simulator => ui.label("Transaction Simulator • scripts and fee estimates (placeholder)"),
			Tab::WebSocket => ui.label("WebSocket Monitor • real-time events (placeholder)"),
			Tab::Analytics => ui.label("Analytics • charts and trends (placeholder)"),
			Tab::Settings => ui.label("Settings • network, theme, language (placeholder)"),
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
