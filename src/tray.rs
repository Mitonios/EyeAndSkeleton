//! System Tray Icon cho Blink Reminder
//!
//! Sử dụng std::sync::mpsc để giao tiếp với main thread.

use std::sync::mpsc;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::*;
use windows::core::{PCWSTR, w};
use std::sync::OnceLock;
use anyhow::Result;

/// Events sent from tray to main thread
#[derive(Debug, Clone)]
pub enum TrayEvent {
    ShowConfig,
    Exit,
}

/// Global sender để window procedure có thể gửi events
static TRAY_SENDER: OnceLock<mpsc::Sender<TrayEvent>> = OnceLock::new();

/// Custom window message for tray icon
const WM_TRAYICON: u32 = WM_USER + 100;

/// Window procedure để handle tray messages
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let mouse_msg = lparam.0 as u32;

            match mouse_msg {
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    log::info!("Tray: Right-click detected, showing context menu");
                    show_context_menu(hwnd);
                }
                WM_LBUTTONUP => {
                    log::info!("Tray: Left-click detected");
                    if let Some(tx) = TRAY_SENDER.get() {
                        if let Err(e) = tx.send(TrayEvent::ShowConfig) {
                            log::error!("Failed to send ShowConfig event: {}", e);
                        }
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let menu_id = (wparam.0 & 0xFFFF) as u32;
            log::info!("Tray: WM_COMMAND received, menu_id={}", menu_id);

            if let Some(tx) = TRAY_SENDER.get() {
                let event = match menu_id {
                    1 => Some(TrayEvent::ShowConfig),
                    2 => Some(TrayEvent::Exit),
                    _ => None,
                };

                if let Some(event) = event {
                    log::info!("Tray: Sending event {:?}", event);
                    if let Err(e) = tx.send(event) {
                        log::error!("Failed to send tray event: {}", e);
                    }
                }
            } else {
                log::error!("TRAY_SENDER not initialized!");
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Hiển thị context menu
unsafe fn show_context_menu(hwnd: HWND) {
    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);

    let hmenu = CreatePopupMenu().unwrap_or_default();
    if hmenu.is_invalid() {
        log::error!("Failed to create popup menu");
        return;
    }

    let _ = AppendMenuW(hmenu, MF_STRING, 1, w!("Cấu hình..."));
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(hmenu, MF_STRING, 2, w!("Thoát"));

    let _ = SetForegroundWindow(hwnd);

    let _ = TrackPopupMenu(
        hmenu,
        TPM_BOTTOMALIGN | TPM_LEFTALIGN,
        point.x,
        point.y,
        Some(0),
        hwnd,
        None,
    );

    let _ = DestroyMenu(hmenu);
}

/// Khởi tạo tray icon và trả về receiver để nhận events
/// Tray chạy trong thread riêng, giao tiếp qua std::sync::mpsc
pub fn init_tray() -> Result<mpsc::Receiver<TrayEvent>> {
    let (tx, rx) = mpsc::channel();

    // Set global sender
    TRAY_SENDER.set(tx).map_err(|_| anyhow::anyhow!("TRAY_SENDER already initialized"))?;

    // Channel để đợi tray thread khởi tạo xong
    let (ready_tx, ready_rx) = mpsc::channel();

    std::thread::spawn(move || {
        log::info!("Tray thread started");

        unsafe {
            // Đăng ký window class
            let class_name = w!("BlinkReminderTrayClass");
            let wc = WNDCLASSW {
                style: WNDCLASS_STYLES(0),
                lpfnWndProc: Some(tray_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: HINSTANCE::default(),
                hIcon: HICON::default(),
                hCursor: HCURSOR::default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
            };

            RegisterClassW(&wc);

            // Tạo hidden window
            let message_window = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                w!("BlinkReminderTray"),
                WS_OVERLAPPED,
                0, 0, 1, 1,
                Some(HWND::default()),
                Some(HMENU::default()),
                Some(HINSTANCE::default()),
                None,
            );

            let Ok(message_window) = message_window else {
                log::error!("Failed to create tray message window");
                let _ = ready_tx.send(false);
                return;
            };

            // Setup tray icon
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = message_window;
            nid.uID = 1;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;

            // Load icon
            nid.hIcon = load_icon();

            // Set tooltip
            let tooltip = "Blink Reminder";
            let tooltip_utf16: Vec<u16> = tooltip.encode_utf16().chain(std::iter::once(0)).collect();
            let copy_len = tooltip_utf16.len().min(128);
            nid.szTip[..copy_len].copy_from_slice(&tooltip_utf16[..copy_len]);

            // Add to tray
            if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                log::error!("Shell_NotifyIconW failed");
                let _ = ready_tx.send(false);
                return;
            }

            log::info!("Tray icon added successfully");
            let _ = ready_tx.send(true);

            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Cleanup
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            let _ = DestroyWindow(message_window);
        }

        log::info!("Tray thread ended");
    });

    // Đợi tray khởi tạo xong
    match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(true) => {
            log::info!("Tray initialized successfully");
            Ok(rx)
        }
        Ok(false) => Err(anyhow::anyhow!("Tray initialization failed")),
        Err(_) => Err(anyhow::anyhow!("Tray initialization timeout")),
    }
}

/// Load icon từ file hoặc dùng system icon
fn load_icon() -> HICON {
    // Thử load từ file cạnh executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let icon_path = exe_dir.join("icon.ico");
            if icon_path.exists() {
                let path_wide: Vec<u16> = icon_path.to_string_lossy()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();

                if let Ok(handle) = unsafe {
                    LoadImageW(
                        Some(HINSTANCE::default()),
                        PCWSTR::from_raw(path_wide.as_ptr()),
                        IMAGE_ICON,
                        16, 16,
                        LR_LOADFROMFILE,
                    )
                } {
                    log::info!("Loaded icon from: {}", icon_path.display());
                    return HICON(handle.0);
                }
            }
        }
    }

    // Fallback to system icon
    log::warn!("Using system default icon");
    unsafe { LoadIconW(Some(HINSTANCE::default()), IDI_APPLICATION).unwrap_or_default() }
}
