//! Native Win32 Config Dialog cho Blink Reminder
//! 
//! Sử dụng Win32 API trực tiếp để tạo config dialog,
//! tránh conflict với tokio/winit event loops.

use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::*;
use windows::core::{PCWSTR, w};
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;
use crate::config::AppConfig;
use crate::overlay;
use crate::registry;
use crate::timer;

/// Dialog control IDs
const ID_CHECKBOX_STARTUP: i32 = 101;
const ID_COMBO_BLINK: i32 = 102;
const ID_COMBO_STANDUP: i32 = 103;
const ID_BTN_TEST_BLINK: i32 = 104;
const ID_BTN_TEST_STANDUP: i32 = 105;
const ID_BTN_SAVE: i32 = 106;
const ID_BTN_CLOSE: i32 = 107;
const ID_LABEL_BLINK: i32 = 108;
const ID_LABEL_STANDUP: i32 = 109;
const ID_LABEL_BLINK_COUNTDOWN: i32 = 110;
const ID_LABEL_STANDUP_COUNTDOWN: i32 = 111;

/// Timer ID for countdown update
const TIMER_ID_COUNTDOWN: usize = 1;

/// Blink interval options (phút)
const BLINK_OPTIONS: [u32; 4] = [1, 5, 10, 30];
/// Stand-up interval options (phút)
const STANDUP_OPTIONS: [u32; 3] = [30, 45, 60];

/// Global state cho dialog
static DIALOG_CONFIG: OnceLock<std::sync::Mutex<ConfigDialogState>> = OnceLock::new();

struct ConfigDialogState {
    config: AppConfig,
    config_tx: Option<mpsc::Sender<Arc<AppConfig>>>,
}

/// Format seconds to mm:ss
fn format_countdown(secs: u64) -> String {
    let mins = secs / 60;
    let remaining_secs = secs % 60;
    format!("{:02}:{:02}", mins, remaining_secs)
}

/// Window procedure cho config dialog
unsafe extern "system" fn config_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            create_controls(hwnd);
            load_config_to_controls(hwnd);
            // Start countdown timer (1 second interval)
            SetTimer(hwnd, TIMER_ID_COUNTDOWN, 1000, None);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_ID_COUNTDOWN {
                update_countdown_labels(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as i32;
            let _notification = ((wparam.0 >> 16) & 0xFFFF) as u32;
            
            match control_id {
                ID_BTN_TEST_BLINK => {
                    log::info!("Test Blink clicked");
                    std::thread::spawn(|| {
                        let _ = overlay::show_overlay(overlay::OverlayType::Blink);
                    });
                }
                ID_BTN_TEST_STANDUP => {
                    log::info!("Test Stand Up clicked");
                    std::thread::spawn(|| {
                        let _ = overlay::show_overlay(overlay::OverlayType::StandUp);
                    });
                }
                ID_BTN_SAVE => {
                    log::info!("Save clicked");
                    save_config_from_controls(hwnd);
                }
                ID_BTN_CLOSE => {
                    log::info!("Close clicked");
                    let _ = KillTimer(hwnd, TIMER_ID_COUNTDOWN);
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = KillTimer(hwnd, TIMER_ID_COUNTDOWN);
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Tạo các controls trong dialog
unsafe fn create_controls(hwnd: HWND) {
    let hinstance = HINSTANCE::default();
    
    // Font mặc định
    let hfont = GetStockObject(DEFAULT_GUI_FONT);
    
    // Checkbox: Run on startup
    let checkbox = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Chạy khi Windows khởi động"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        20, 20, 250, 25,
        hwnd,
        HMENU(ID_CHECKBOX_STARTUP as _),
        hinstance,
        None,
    );
    SendMessageW(checkbox, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // === BLINK SECTION ===
    // Label: Blink interval
    let label_blink = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("Nhắc chớp mắt (phút):"),
        WS_CHILD | WS_VISIBLE,
        20, 60, 130, 20,
        hwnd,
        HMENU(ID_LABEL_BLINK as _),
        hinstance,
        None,
    );
    SendMessageW(label_blink, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Combo: Blink interval
    let combo_blink = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        155, 55, 55, 120,
        hwnd,
        HMENU(ID_COMBO_BLINK as _),
        hinstance,
        None,
    );
    SendMessageW(combo_blink, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Add blink options
    for opt in BLINK_OPTIONS {
        let text: Vec<u16> = format!("{}", opt).encode_utf16().chain(std::iter::once(0)).collect();
        SendMessageW(combo_blink, CB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as _));
    }
    
    // Label: Blink countdown
    let label_blink_countdown = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("⏱️ --:--"),
        WS_CHILD | WS_VISIBLE,
        220, 60, 70, 20,
        hwnd,
        HMENU(ID_LABEL_BLINK_COUNTDOWN as _),
        hinstance,
        None,
    );
    SendMessageW(label_blink_countdown, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // === STANDUP SECTION ===
    // Label: Stand-up interval
    let label_standup = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("Nhắc đứng dậy (phút):"),
        WS_CHILD | WS_VISIBLE,
        20, 95, 130, 20,
        hwnd,
        HMENU(ID_LABEL_STANDUP as _),
        hinstance,
        None,
    );
    SendMessageW(label_standup, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Combo: Stand-up interval
    let combo_standup = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        155, 90, 55, 120,
        hwnd,
        HMENU(ID_COMBO_STANDUP as _),
        hinstance,
        None,
    );
    SendMessageW(combo_standup, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Add stand-up options
    for opt in STANDUP_OPTIONS {
        let text: Vec<u16> = format!("{}", opt).encode_utf16().chain(std::iter::once(0)).collect();
        SendMessageW(combo_standup, CB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as _));
    }
    
    // Label: Standup countdown
    let label_standup_countdown = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("⏱️ --:--"),
        WS_CHILD | WS_VISIBLE,
        220, 95, 70, 20,
        hwnd,
        HMENU(ID_LABEL_STANDUP_COUNTDOWN as _),
        hinstance,
        None,
    );
    SendMessageW(label_standup_countdown, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // === BUTTONS ===
    // Button: Test Blink
    let btn_test_blink = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Test Chớp mắt"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        20, 140, 120, 30,
        hwnd,
        HMENU(ID_BTN_TEST_BLINK as _),
        hinstance,
        None,
    );
    SendMessageW(btn_test_blink, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Button: Test Stand Up
    let btn_test_standup = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Test Đứng dậy"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        150, 140, 120, 30,
        hwnd,
        HMENU(ID_BTN_TEST_STANDUP as _),
        hinstance,
        None,
    );
    SendMessageW(btn_test_standup, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Button: Save
    let btn_save = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Lưu"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        70, 200, 80, 30,
        hwnd,
        HMENU(ID_BTN_SAVE as _),
        hinstance,
        None,
    );
    SendMessageW(btn_save, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
    
    // Button: Close
    let btn_close = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Đóng"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        160, 200, 80, 30,
        hwnd,
        HMENU(ID_BTN_CLOSE as _),
        hinstance,
        None,
    );
    SendMessageW(btn_close, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
}

/// Update countdown labels
unsafe fn update_countdown_labels(hwnd: HWND) {
    if let Some(info) = timer::get_timer_info() {
        // Update blink countdown
        let blink_secs = info.blink_remaining_secs();
        let blink_text = format!("⏱️ {}", format_countdown(blink_secs));
        let blink_wide: Vec<u16> = blink_text.encode_utf16().chain(std::iter::once(0)).collect();
        let blink_label = GetDlgItem(hwnd, ID_LABEL_BLINK_COUNTDOWN);
        let _ = SetWindowTextW(blink_label, PCWSTR::from_raw(blink_wide.as_ptr()));
        
        // Update standup countdown
        let standup_secs = info.standup_remaining_secs();
        let standup_text = format!("⏱️ {}", format_countdown(standup_secs));
        let standup_wide: Vec<u16> = standup_text.encode_utf16().chain(std::iter::once(0)).collect();
        let standup_label = GetDlgItem(hwnd, ID_LABEL_STANDUP_COUNTDOWN);
        let _ = SetWindowTextW(standup_label, PCWSTR::from_raw(standup_wide.as_ptr()));
    }
}

/// Load config values vào controls
unsafe fn load_config_to_controls(hwnd: HWND) {
    let state = DIALOG_CONFIG.get().and_then(|m| m.lock().ok());
    if let Some(state) = state {
        let config = &state.config;
        
        // Checkbox startup
        let checkbox = GetDlgItem(hwnd, ID_CHECKBOX_STARTUP);
        SendMessageW(checkbox, BM_SETCHECK, WPARAM(if config.startup { BST_CHECKED.0 as _ } else { BST_UNCHECKED.0 as _ }), LPARAM(0));
        
        // Combo blink interval
        let combo_blink = GetDlgItem(hwnd, ID_COMBO_BLINK);
        let blink_idx = BLINK_OPTIONS.iter().position(|&x| x == config.blink_interval).unwrap_or(1);
        SendMessageW(combo_blink, CB_SETCURSEL, WPARAM(blink_idx), LPARAM(0));
        
        // Combo stand-up interval
        let combo_standup = GetDlgItem(hwnd, ID_COMBO_STANDUP);
        let standup_idx = STANDUP_OPTIONS.iter().position(|&x| x == config.standup_interval).unwrap_or(1);
        SendMessageW(combo_standup, CB_SETCURSEL, WPARAM(standup_idx), LPARAM(0));
    }
    
    // Initial countdown update
    update_countdown_labels(hwnd);
}

/// Lưu config từ controls
unsafe fn save_config_from_controls(hwnd: HWND) {
    // Đọc values từ controls
    let checkbox = GetDlgItem(hwnd, ID_CHECKBOX_STARTUP);
    let startup = SendMessageW(checkbox, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == BST_CHECKED.0 as isize;
    
    let combo_blink = GetDlgItem(hwnd, ID_COMBO_BLINK);
    let blink_idx = SendMessageW(combo_blink, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
    let blink_interval = BLINK_OPTIONS.get(blink_idx).copied().unwrap_or(5);
    
    let combo_standup = GetDlgItem(hwnd, ID_COMBO_STANDUP);
    let standup_idx = SendMessageW(combo_standup, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
    let standup_interval = STANDUP_OPTIONS.get(standup_idx).copied().unwrap_or(45);
    
    // Tạo config mới
    let new_config = AppConfig {
        startup,
        blink_interval,
        standup_interval,
    };
    
    log::info!("Saving config: {:?}", new_config);
    
    // Lưu vào file
    if let Err(e) = crate::config::save_config(&new_config) {
        log::error!("Failed to save config: {}", e);
        MessageBoxW(
            hwnd,
            w!("Không thể lưu cấu hình!"),
            w!("Lỗi"),
            MB_OK | MB_ICONERROR,
        );
        return;
    }
    
    // Update registry
    if let Err(e) = registry::set_startup(startup) {
        log::error!("Failed to update registry: {}", e);
    }
    
    // Gửi config mới đến main thread
    if let Some(state) = DIALOG_CONFIG.get().and_then(|m| m.lock().ok()) {
        if let Some(tx) = &state.config_tx {
            let _ = tx.try_send(Arc::new(new_config.clone()));
        }
    }
    
    // Update state
    if let Some(mut state) = DIALOG_CONFIG.get().and_then(|m| m.lock().ok()) {
        state.config = new_config;
    }
    
    // Show success
    MessageBoxW(
        hwnd,
        w!("Đã lưu cấu hình thành công!\nTimer sẽ được reset."),
        w!("Thông báo"),
        MB_OK | MB_ICONINFORMATION,
    );
}

/// Hiển thị config dialog
pub fn show_config_dialog(config: Arc<AppConfig>, config_tx: Option<mpsc::Sender<Arc<AppConfig>>>) {
    // Initialize global state
    let _ = DIALOG_CONFIG.set(std::sync::Mutex::new(ConfigDialogState {
        config: (*config).clone(),
        config_tx,
    }));
    
    // Spawn thread để tạo và chạy dialog
    std::thread::spawn(move || {
        unsafe {
            // Đăng ký window class
            let class_name = w!("BlinkReminderConfigClass");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(config_dialog_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: HINSTANCE::default(),
                hIcon: HICON::default(),
                hCursor: LoadCursorW(HINSTANCE::default(), IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as _),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
            };
            
            RegisterClassW(&wc);
            
            // Tính vị trí center screen
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            let dialog_width = 310;
            let dialog_height = 280;
            let x = (screen_width - dialog_width) / 2;
            let y = (screen_height - dialog_height) / 2;
            
            // Tạo window
            let hwnd = CreateWindowExW(
                WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
                class_name,
                w!("Blink Reminder - Cấu hình"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                x, y, dialog_width, dialog_height,
                HWND::default(),
                HMENU::default(),
                HINSTANCE::default(),
                None,
            );
            
            if hwnd == HWND::default() {
                log::error!("Failed to create config dialog");
                return;
            }
            
            // Show window
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
            
            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                if !IsDialogMessageW(hwnd, &msg).as_bool() {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    });
}
