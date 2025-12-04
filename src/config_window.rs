use crate::config::{save_config, AppConfig};
use crate::overlay::OverlayType;
use anyhow::Result;
use eframe::egui;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Struct để lưu trạng thái của config window
pub struct ConfigWindow {
    config: AppConfig,
    original_config: AppConfig,
    blink_options: Vec<u32>,
    standup_options: Vec<u32>,
    should_close: bool,
}

impl ConfigWindow {
    /// Tạo config window mới với config hiện tại
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self {
            config: (*config).clone(),
            original_config: (*config).clone(),
            blink_options: vec![1, 5, 10, 30],
            standup_options: vec![30, 45, 60],
            should_close: false,
        }
    }

    /// Kiểm tra xem config có thay đổi không
    fn has_changes(&self) -> bool {
        self.config.startup != self.original_config.startup
            || self.config.blink_interval != self.original_config.blink_interval
            || self.config.standup_interval != self.original_config.standup_interval
    }

}

impl eframe::App for ConfigWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.should_close {
            // In eframe, we can't directly close from update.
            // The window will close when this returns and should_close is true
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Blink Reminder Settings");

            ui.separator();

            // Checkbox startup
            ui.horizontal(|ui| {
                ui.label("Run when Windows starts:");
                ui.checkbox(&mut self.config.startup, "");
            });

            ui.separator();

            // Blink interval dropdown
            ui.horizontal(|ui| {
                ui.label("Blink reminder interval:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{} minutes", self.config.blink_interval))
                    .show_ui(ui, |ui| {
                        for &option in &self.blink_options {
                            ui.selectable_value(
                                &mut self.config.blink_interval,
                                option,
                                format!("{} minutes", option),
                            );
                        }
                    });
            });

            // Stand-up interval dropdown
            ui.horizontal(|ui| {
                ui.label("Stand-up reminder interval:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{} minutes", self.config.standup_interval))
                    .show_ui(ui, |ui| {
                        for &option in &self.standup_options {
                            ui.selectable_value(
                                &mut self.config.standup_interval,
                                option,
                                format!("{} minutes", option),
                            );
                        }
                    });
            });

            ui.separator();

            // Buttons row
            ui.horizontal(|ui| {
                // Test buttons
                if ui.button("Test Blink").clicked() {
                    log::info!("Test Blink button clicked");
                    if let Err(e) = crate::overlay::show_overlay(OverlayType::Blink) {
                        log::error!("Lỗi khi test blink: {}", e);
                    }
                }

                if ui.button("Test Stand Up").clicked() {
                    log::info!("Test Stand Up button clicked");
                    if let Err(e) = crate::overlay::show_overlay(OverlayType::StandUp) {
                        log::error!("Lỗi khi test stand up: {}", e);
                    }
                }

                ui.separator();

                // Save button - chỉ enable nếu có thay đổi
                let save_enabled = self.has_changes();
                if ui
                    .add_enabled(save_enabled, egui::Button::new("Save"))
                    .clicked()
                {
                    log::info!("Save button clicked");
                    match save_config(&self.config) {
                        Ok(_) => {
                            log::info!("Config đã được lưu thành công");
                            self.original_config = self.config.clone();
                        }
                        Err(e) => {
                            log::error!("Lỗi khi lưu config: {}", e);
                        }
                    }
                }

                // Close button
                if ui.button("Close").clicked() {
                    log::info!("Close button clicked");
                    self.should_close = true;
                }
            });

            // Hiển thị trạng thái
            ui.separator();
            if self.has_changes() {
                ui.colored_label(egui::Color32::YELLOW, "⚠ You have unsaved changes");
            } else {
                ui.colored_label(egui::Color32::GREEN, "✓ All changes saved");
            }
        });
    }

}

/// Hiển thị config window với khả năng gửi config updates
pub fn show_config_window_with_updates(
    config: Arc<AppConfig>,
    config_update_tx: mpsc::Sender<Arc<AppConfig>>,
) -> Result<()> {
    let app = ConfigWindowWithUpdates::new(config, config_update_tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_resizable(false)
            .with_title("Blink Reminder - Settings"),
        ..Default::default()
    };

    eframe::run_native(
        "Blink Reminder Settings",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|e| anyhow::anyhow!("Không thể mở config window: {}", e))?;

    Ok(())
}


/// Config window với khả năng gửi updates
pub struct ConfigWindowWithUpdates {
    inner: ConfigWindow,
    config_update_tx: mpsc::Sender<Arc<AppConfig>>,
}

impl ConfigWindowWithUpdates {
    pub fn new(config: Arc<AppConfig>, config_update_tx: mpsc::Sender<Arc<AppConfig>>) -> Self {
        Self {
            inner: ConfigWindow::new(config),
            config_update_tx,
        }
    }
}

impl eframe::App for ConfigWindowWithUpdates {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Check if we should close
            if self.inner.should_close {
                ui.label("Closing window...");
                return;
            }
            ui.heading("Blink Reminder Settings");

            ui.separator();

            // Checkbox startup
            ui.horizontal(|ui| {
                ui.label("Run when Windows starts:");
                ui.checkbox(&mut self.inner.config.startup, "");
            });

            ui.separator();

            // Blink interval dropdown
            ui.horizontal(|ui| {
                ui.label("Blink reminder interval:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{} minutes", self.inner.config.blink_interval))
                    .show_ui(ui, |ui| {
                        for &option in &self.inner.blink_options {
                            ui.selectable_value(
                                &mut self.inner.config.blink_interval,
                                option,
                                format!("{} minutes", option),
                            );
                        }
                    });
            });

            // Stand-up interval dropdown
            ui.horizontal(|ui| {
                ui.label("Stand-up reminder interval:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{} minutes", self.inner.config.standup_interval))
                    .show_ui(ui, |ui| {
                        for &option in &self.inner.standup_options {
                            ui.selectable_value(
                                &mut self.inner.config.standup_interval,
                                option,
                                format!("{} minutes", option),
                            );
                        }
                    });
            });

            ui.separator();

            // Buttons row
            ui.horizontal(|ui| {
                // Test buttons
                if ui.button("Test Blink").clicked() {
                    log::info!("Test Blink button clicked");
                    if let Err(e) = crate::overlay::show_overlay(OverlayType::Blink) {
                        log::error!("Lỗi khi test blink: {}", e);
                    }
                }

                if ui.button("Test Stand Up").clicked() {
                    log::info!("Test Stand Up button clicked");
                    if let Err(e) = crate::overlay::show_overlay(OverlayType::StandUp) {
                        log::error!("Lỗi khi test stand up: {}", e);
                    }
                }

                ui.separator();

                // Save button - chỉ enable nếu có thay đổi
                let save_enabled = self.inner.has_changes();
                if ui
                    .add_enabled(save_enabled, egui::Button::new("Save"))
                    .clicked()
                {
                    log::info!("Save button clicked");
                    match save_config(&self.inner.config) {
                        Ok(_) => {
                            log::info!("Config đã được lưu thành công");
                            self.inner.original_config = self.inner.config.clone();

                            // Send config update to main thread
                            let config_arc = Arc::new(self.inner.config.clone());
                            let tx = self.config_update_tx.clone();
                            tokio::spawn(async move {
                                if let Err(e) = tx.send(config_arc).await {
                                    log::error!("Không thể gửi config update: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            log::error!("Lỗi khi lưu config: {}", e);
                        }
                    }
                }

                // Close button
                if ui.button("Close").clicked() {
                    log::info!("Close button clicked");
                    self.inner.should_close = true;
                }
            });

            // Hiển thị trạng thái
            ui.separator();
            if self.inner.has_changes() {
                ui.colored_label(egui::Color32::YELLOW, "⚠ You have unsaved changes");
            } else {
                ui.colored_label(egui::Color32::GREEN, "✓ All changes saved");
            }
        });
    }
}
