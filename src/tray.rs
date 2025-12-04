use tokio::sync::mpsc;
use tray_item::{IconSource, TrayItem};
use anyhow::Result;

/// Events sent from tray to main thread
#[derive(Debug)]
pub enum TrayEvent {
    ShowConfig,
    Exit,
}

/// Khởi tạo system tray icon và menu
pub fn init_tray(tx: mpsc::Sender<TrayEvent>) -> Result<()> {
    // Tạo tray item với icon mặc định
    let mut tray = TrayItem::new(
        "Blink Reminder",
        IconSource::Resource("default"),
    ).map_err(|e| anyhow::anyhow!("Không thể tạo tray icon: {}", e))?;

    // Thêm menu items
    let tx_config = tx.clone();
    tray.add_menu_item("Config...", move || {
        log::info!("Menu Config được click");
        let _ = tx_config.try_send(TrayEvent::ShowConfig);
    }).map_err(|e| anyhow::anyhow!("Không thể thêm menu Config: {}", e))?;

    let tx_exit = tx.clone();
    tray.add_menu_item("Exit", move || {
        log::info!("Menu Exit được click");
        let _ = tx_exit.try_send(TrayEvent::Exit);
    }).map_err(|e| anyhow::anyhow!("Không thể thêm menu Exit: {}", e))?;

    // Lưu tray vào static để tránh bị drop
    static mut TRAY: Option<TrayItem> = None;
    unsafe {
        TRAY = Some(tray);
    }

    log::info!("Đã khởi tạo system tray icon");
    Ok(())
}

/// Test function để kiểm tra tray functionality
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_tray_events() {
        let (tx, mut rx) = mpsc::channel(10);

        // Test event sending (không thể test init_tray vì cần GUI)
        let _ = tx.try_send(TrayEvent::ShowConfig);
        let _ = tx.try_send(TrayEvent::Exit);

        // Verify events
        if let Some(event) = rx.recv().await {
            assert!(matches!(event, TrayEvent::ShowConfig));
        }

        if let Some(event) = rx.recv().await {
            assert!(matches!(event, TrayEvent::Exit));
        }
    }
}
