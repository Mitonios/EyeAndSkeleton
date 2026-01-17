//! Config Window sử dụng egui/eframe
//!
//! Chạy trên main thread, giao tiếp với background threads qua std channels.

use crate::config::{AppConfig, OverlayPosition};
use crate::overlay;
use crate::timer;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;
use std::sync::mpsc as std_mpsc;
use tokio::sync::mpsc as tokio_mpsc;

/// Blink interval options (phút)
const BLINK_OPTIONS: [u32; 4] = [1, 5, 10, 30];
/// Stand-up interval options (phút)
const STANDUP_OPTIONS: [u32; 3] = [30, 45, 60];

/// Metadata từ Cargo
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// Messages từ background thread đến UI
#[derive(Debug, Clone)]
pub enum UiMessage {
    /// Yêu cầu mở/focus config window
    ShowConfig,
    /// Yêu cầu đóng app
    Exit,
}

/// Config App state cho egui
pub struct ConfigApp {
    /// Config hiện tại
    config: AppConfig,
    /// Selected index cho blink combo
    blink_idx: usize,
    /// Selected index cho standup combo
    standup_idx: usize,
    /// Selected index cho position combo
    position_idx: usize,
    /// Channel để gửi config updates đến tokio (tokio channel)
    config_tx: Option<tokio_mpsc::Sender<Arc<AppConfig>>>,
    /// Channel để nhận messages từ tray (std channel - non-blocking)
    ui_rx: Option<std_mpsc::Receiver<UiMessage>>,
    /// Flag để request exit
    should_exit: bool,
}

impl ConfigApp {
    /// Tạo ConfigApp mới
    pub fn new(
        config: AppConfig,
        config_tx: Option<tokio_mpsc::Sender<Arc<AppConfig>>>,
        ui_rx: Option<std_mpsc::Receiver<UiMessage>>,
    ) -> Self {
        let blink_idx = BLINK_OPTIONS
            .iter()
            .position(|&x| x == config.blink_interval)
            .unwrap_or(1);
        let standup_idx = STANDUP_OPTIONS
            .iter()
            .position(|&x| x == config.standup_interval)
            .unwrap_or(1);
        let position_idx = config.overlay_position.index();

        Self {
            config,
            blink_idx,
            standup_idx,
            position_idx,
            config_tx,
            ui_rx,
            should_exit: false,
        }
    }

    /// Lưu config và gửi update
    fn save_and_notify(&mut self) {
        self.config.blink_interval = BLINK_OPTIONS[self.blink_idx];
        self.config.standup_interval = STANDUP_OPTIONS[self.standup_idx];
        self.config.overlay_position = OverlayPosition::from_index(self.position_idx);

        log::info!("Saving config: {:?}", self.config);

        if let Err(e) = crate::config::save_config(&self.config) {
            log::error!("Failed to save config: {}", e);
            return;
        }

        if let Some(tx) = &self.config_tx {
            let _ = tx.try_send(Arc::new(self.config.clone()));
        }
    }

    /// Format countdown mm:ss
    fn format_countdown(secs: u64) -> String {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("{:02}:{:02}", mins, remaining_secs)
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll UI messages từ tray (std::sync::mpsc - non-blocking)
        if let Some(rx) = &self.ui_rx {
            // Try to receive all pending messages
            while let Ok(msg) = rx.try_recv() {
                log::info!("UI received message: {:?}", msg);
                match msg {
                    UiMessage::ShowConfig => {
                        log::info!("ShowConfig: Making window visible and focused");
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    UiMessage::Exit => {
                        log::info!("Exit: Setting should_exit = true");
                        self.should_exit = true;
                    }
                }
            }
        }

        // Exit nếu được yêu cầu
        if self.should_exit {
            log::info!("Exiting app...");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Request repaint thường xuyên hơn để poll messages
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Blink Reminder - Cấu hình");
            ui.add_space(10.0);

            // === BLINK SECTION ===
            ui.horizontal(|ui| {
                ui.label("Nhắc chớp mắt (phút):");

                let blink_changed = egui::ComboBox::from_id_salt("blink_combo")
                    .selected_text(format!("{}", BLINK_OPTIONS[self.blink_idx]))
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for (i, &opt) in BLINK_OPTIONS.iter().enumerate() {
                            if ui.selectable_value(&mut self.blink_idx, i, format!("{}", opt)).changed() {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);

                if let Some(info) = timer::get_timer_info() {
                    ui.label(Self::format_countdown(info.blink_remaining_secs()));
                } else {
                    ui.label("--:--");
                }

                if ui.button("Test").clicked() {
                    log::info!("Test Blink clicked");
                    std::thread::spawn(|| {
                        let _ = overlay::show_overlay(overlay::OverlayType::Blink);
                    });
                }

                if blink_changed {
                    self.save_and_notify();
                }
            });

            ui.add_space(5.0);

            // === STANDUP SECTION ===
            ui.horizontal(|ui| {
                ui.label("Nhắc đứng dậy (phút):");

                let standup_changed = egui::ComboBox::from_id_salt("standup_combo")
                    .selected_text(format!("{}", STANDUP_OPTIONS[self.standup_idx]))
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for (i, &opt) in STANDUP_OPTIONS.iter().enumerate() {
                            if ui.selectable_value(&mut self.standup_idx, i, format!("{}", opt)).changed() {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);

                if let Some(info) = timer::get_timer_info() {
                    ui.label(Self::format_countdown(info.standup_remaining_secs()));
                } else {
                    ui.label("--:--");
                }

                if ui.button("Test").clicked() {
                    log::info!("Test Stand Up clicked");
                    std::thread::spawn(|| {
                        let _ = overlay::show_overlay(overlay::OverlayType::StandUp);
                    });
                }

                if standup_changed {
                    self.save_and_notify();
                }
            });

            ui.add_space(5.0);

            // === POSITION SECTION ===
            ui.horizontal(|ui| {
                ui.label("Vị trí thông báo:");

                let positions = OverlayPosition::all();
                let position_changed = egui::ComboBox::from_id_salt("position_combo")
                    .selected_text(positions[self.position_idx].display_name())
                    .show_ui(ui, |ui| {
                        let mut changed = false;
                        for (i, pos) in positions.iter().enumerate() {
                            if ui.selectable_value(&mut self.position_idx, i, pos.display_name()).changed() {
                                changed = true;
                            }
                        }
                        changed
                    })
                    .inner
                    .unwrap_or(false);

                if position_changed {
                    self.save_and_notify();
                }
            });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(5.0);

            // === VERSION INFO + QUIT BUTTON ===
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(format!("Phiên bản: {}", APP_VERSION));
                    ui.label(format!("Tác giả: {}", APP_AUTHORS));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Thoát").clicked() {
                        log::info!("Quit button clicked");
                        self.should_exit = true;
                    }
                });
            });
        });

        // Handle close button - minimize to tray instead of closing (unless exit requested)
        if !self.should_exit && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }
}

/// Chạy config window với eframe trên main thread
pub fn run_config_window(
    config: AppConfig,
    config_tx: Option<tokio_mpsc::Sender<Arc<AppConfig>>>,
    ui_rx: Option<std_mpsc::Receiver<UiMessage>>,
) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Blink Reminder - Cấu hình")
            .with_inner_size([380.0, 200.0])
            .with_min_inner_size([350.0, 180.0])
            .with_resizable(true),
        centered: true,
        ..Default::default()
    };

    log::info!("Starting eframe on main thread");

    eframe::run_native(
        "Blink Reminder",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(ConfigApp::new(config, config_tx, ui_rx)))
        }),
    )
}

/// Setup custom fonts để hỗ trợ tiếng Việt
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let font_paths = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\tahoma.ttf",
    ];

    for font_path in font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            log::info!("Loaded font from: {}", font_path);

            fonts.font_data.insert(
                "vietnamese_font".to_owned(),
                Arc::new(FontData::from_owned(font_data)),
            );

            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "vietnamese_font".to_owned());

            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "vietnamese_font".to_owned());

            break;
        }
    }

    ctx.set_fonts(fonts);
}
