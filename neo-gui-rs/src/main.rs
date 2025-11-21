use eframe::{egui, egui::RichText};
use once_cell::sync::Lazy;

static VERSION: Lazy<String> = Lazy::new(|| "0.5.1".to_string());

fn main() -> eframe::Result<()> {
	let native_options = eframe::NativeOptions {
		follow_system_theme: true,
		..Default::default()
	};

	eframe::run_native(
		"NeoRust GUI (Native)",
		native_options,
		Box::new(|_cc| Box::new(NeoGuiApp::default())),
	)
}

#[derive(Default)]
struct NeoGuiApp {
	current_tab: Tab,
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

impl NeoGuiApp {
	fn render_sidebar(&mut self, ui: &mut egui::Ui) {
		ui.heading(RichText::new("NeoRust SDK").strong());
		ui.label(format!("Native GUI · v{}", *VERSION));
		ui.separator();

		self.tab_button(ui, Tab::Dashboard, "Dashboard");
		self.tab_button(ui, Tab::Wallet, "Wallet");
		self.tab_button(ui, Tab::HdWallet, "HD Wallet");
		self.tab_button(ui, Tab::Simulator, "Simulator");
		self.tab_button(ui, Tab::WebSocket, "WebSocket Monitor");
		self.tab_button(ui, Tab::Analytics, "Analytics");
		self.tab_button(ui, Tab::Settings, "Settings");

		ui.separator();
		ui.label("Status");
		ui.small("Network: Not connected");
		ui.small("Wallet: Not connected");
	}

	fn tab_button(&mut self, ui: &mut egui::Ui, tab: Tab, label: &str) {
		let selected = matches!(self.current_tab, t if t as u8 == tab as u8);
		if ui.selectable_label(selected, label).clicked() {
			self.current_tab = tab;
		}
	}

	fn render_content(&mut self, ui: &mut egui::Ui) {
		match self.current_tab {
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
