use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::config::AppConfig;

mod config;
mod registry;
mod overlay;
mod timer;
mod tray;
mod config_window;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    log::info!("Khởi động Blink Reminder...");

    // Load configuration
    let config = config::load_config()?;
    let mut config_arc = Arc::new(config);

    // Handle startup registry
    if config_arc.startup {
        registry::ensure_startup()?;
    }

    // Setup tray icon and menu
    let (tray_tx, mut tray_rx) = mpsc::channel(32);
    tray::init_tray(tray_tx)?;

    // Setup timers
    let (timer_tx, mut timer_rx) = mpsc::channel(32);
    let timer_handle = timer::start_timers(config_arc.clone(), timer_tx)?;

    // Channel for config updates from config window
    let (config_update_tx, mut config_update_rx) = mpsc::channel::<Arc<AppConfig>>(32);

    // Main event loop
    loop {
        tokio::select! {
            // Handle config updates
            Some(new_config) = config_update_rx.recv() => {
                log::info!("Nhận config update từ config window");
                // Update registry if startup setting changed
                if new_config.startup != config_arc.startup {
                    match registry::set_startup(new_config.startup) {
                        Ok(_) => log::info!("Đã cập nhật registry startup"),
                        Err(e) => {
                            log::error!("Lỗi khi cập nhật registry startup: {}", e);
                            // Continue anyway - registry failure shouldn't crash the app
                        }
                    }
                }
                // Update timer config
                match timer_handle.update_config(new_config.clone()).await {
                    Ok(_) => log::info!("Đã cập nhật timer config"),
                    Err(e) => {
                        log::error!("Lỗi khi cập nhật timer config: {}", e);
                        // Continue anyway - timer update failure shouldn't crash the app
                    }
                }
                // Update shared config
                config_arc = new_config;
            }

            // Handle tray events
            Some(event) = tray_rx.recv() => {
                match event {
                    tray::TrayEvent::ShowConfig => {
                        // Open config window in separate thread
                        let config_clone = config_arc.clone();
                        let config_update_tx_clone = config_update_tx.clone();
                        std::thread::spawn(move || {
                            match config_window::show_config_window_with_updates(config_clone, config_update_tx_clone) {
                                Ok(_) => log::info!("Config window closed"),
                                Err(e) => log::error!("Lỗi khi mở config window: {}", e),
                            }
                        });
                    }
                    tray::TrayEvent::Exit => {
                        log::info!("Nhận lệnh exit từ tray menu");
                        break;
                    }
                }
            }

            // Handle timer events
            Some(event) = timer_rx.recv() => {
                match event {
                    timer::TimerEvent::ShowBlink => {
                        log::info!("Hiển thị overlay chớp mắt");
                        match overlay::show_overlay(overlay::OverlayType::Blink) {
                            Ok(_) => log::debug!("Overlay blink hiển thị thành công"),
                            Err(e) => log::error!("Lỗi khi hiển thị overlay blink: {}", e),
                        }
                    }
                    timer::TimerEvent::ShowStandUp => {
                        log::info!("Hiển thị overlay đứng dậy");
                        match overlay::show_overlay(overlay::OverlayType::StandUp) {
                            Ok(_) => log::debug!("Overlay stand-up hiển thị thành công"),
                            Err(e) => log::error!("Lỗi khi hiển thị overlay stand-up: {}", e),
                        }
                    }
                }
            }
        }
    }

    // Cleanup
    log::info!("Shutting down Blink Reminder...");
    Ok(())
}
