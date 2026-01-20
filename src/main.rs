// Ẩn console window trên Windows
#![windows_subsystem = "windows"]

use crate::config::AppConfig;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc as tokio_mpsc;

mod config;
mod config_window;
mod debug_logger;
mod overlay;
mod singleton;
mod timer;
mod tray;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    log::info!("Khởi động Blink Reminder...");
    debug_logger::log_event("Khởi động ứng dụng");

    // Kiểm tra singleton - chỉ cho phép 1 instance chạy
    let _singleton_guard = match singleton::acquire_singleton()? {
        Some(guard) => guard,
        None => {
            log::info!("Đã có instance đang chạy, thoát...");
            return Ok(());
        }
    };
    log::info!("Singleton acquired");

    // Load configuration
    let config = config::load_config()?;
    let config_arc = Arc::new(config.clone());

    // === KHỞI TẠO TRAY TRƯỚC (sử dụng std::sync::mpsc) ===
    let tray_rx = tray::init_tray()?;

    // === KHỞI TẠO TOKIO RUNTIME ===
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // Channels cho timer
    let (timer_event_tx, mut timer_event_rx) = tokio_mpsc::channel(32);
    let (timer_update_tx, timer_update_rx) = tokio_mpsc::channel::<Arc<AppConfig>>(32);

    // Channel cho UI messages (từ tray đến egui) - dùng std channel vì egui chạy ngoài tokio
    let (ui_tx, ui_rx) = std::sync::mpsc::channel::<config_window::UiMessage>();

    // Channel cho IPC
    let (ipc_tx, mut ipc_rx) = tokio_mpsc::channel::<()>(32);

    // Tạo IPC window
    let ipc_tx_clone = ipc_tx.clone();
    singleton::create_ipc_window(move || {
        let _ = ipc_tx_clone.try_send(());
    })?;

    // === SPAWN BACKGROUND TASKS ===
    // Clone ui_tx cho IPC handler (std::sync::mpsc::Sender)
    let ui_tx_for_ipc = ui_tx.clone();
    let config_for_timer = config_arc.clone();

    runtime.spawn(async move {
        // Khởi tạo timer manager
        let timer_manager = timer::TimerManager::new(config_for_timer);

        tokio::spawn(async move {
            if let Err(e) = timer_manager.run_with_updates(timer_event_tx, timer_update_rx).await {
                log::error!("Timer manager error: {}", e);
            }
        });

        // Main event loop - xử lý timer events và IPC
        loop {
            tokio::select! {
                Some(_) = ipc_rx.recv() => {
                    log::info!("IPC: Received signal from another instance");
                    // std::sync::mpsc::Sender::send() - không có await
                    let _ = ui_tx_for_ipc.send(config_window::UiMessage::ShowConfig);
                }

                Some(event) = timer_event_rx.recv() => {
                    match event {
                        timer::TimerEvent::ShowBlink => {
                            log::info!("Timer: Show blink overlay");
                            std::thread::spawn(|| {
                                if let Err(e) = overlay::show_overlay(overlay::OverlayType::Blink) {
                                    log::error!("Failed to show blink overlay: {}", e);
                                } else {
                                    debug_logger::log_event("Bắt đầu hiển thị nhắc nhở: Blink");
                                }
                            });
                        }
                        timer::TimerEvent::ShowStandUp => {
                            log::info!("Timer: Show stand up overlay");
                            std::thread::spawn(|| {
                                if let Err(e) = overlay::show_overlay(overlay::OverlayType::StandUp) {
                                    log::error!("Failed to show stand up overlay: {}", e);
                                } else {
                                    debug_logger::log_event("Bắt đầu hiển thị nhắc nhở: StandUp");
                                }
                            });
                        }
                    }
                }
            }
        }
    });

    // === SPAWN TRAY EVENT HANDLER ===
    // Tray sử dụng std::sync::mpsc, cần thread riêng để poll và forward đến UI
    std::thread::spawn(move || {
        log::info!("Tray event handler started");
        loop {
            match tray_rx.recv() {
                Ok(event) => {
                    log::info!("Tray event received: {:?}", event);
                    let msg = match event {
                        tray::TrayEvent::ShowConfig => config_window::UiMessage::ShowConfig,
                        tray::TrayEvent::Exit => config_window::UiMessage::Exit,
                    };
                    // Dùng std::sync::mpsc::Sender::send()
                    if ui_tx.send(msg).is_err() {
                        log::error!("Failed to send UI message from tray");
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Tray channel error: {}", e);
                    break;
                }
            }
        }
        log::info!("Tray event handler ended");
    });

    log::info!("Background tasks started, launching UI on main thread");

    // === CHẠY EGUI TRÊN MAIN THREAD ===
    if let Err(e) = config_window::run_config_window(config, Some(timer_update_tx), Some(ui_rx)) {
        log::error!("eframe error: {}", e);
    }

    // Cleanup
    log::info!("Shutting down Blink Reminder...");
    debug_logger::log_event("Thoát ứng dụng (Log out)");
    Ok(())
}
