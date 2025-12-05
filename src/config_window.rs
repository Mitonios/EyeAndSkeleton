//! Native Win32 Config Dialog cho Blink Reminder
//!
//! Sử dụng Win32 API trực tiếp để tạo config dialog,
//! tránh conflict với tokio/winit event loops.

use crate::config::AppConfig;
use crate::overlay;
use crate::registry;
use crate::timer;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Dialog control IDs
const ID_CHECKBOX_STARTUP: i32 = 101;
const ID_COMBO_BLINK: i32 = 102;
const ID_COMBO_STANDUP: i32 = 103;
const ID_BTN_TEST_BLINK: i32 = 104;
const ID_BTN_TEST_STANDUP: i32 = 105;
const ID_BTN_CLOSE: i32 = 107;
const ID_LABEL_BLINK: i32 = 108;
const ID_LABEL_STANDUP: i32 = 109;
const ID_LABEL_BLINK_COUNTDOWN: i32 = 110;
const ID_LABEL_STANDUP_COUNTDOWN: i32 = 111;
const ID_COMBO_POSITION: i32 = 112;
const ID_LABEL_POSITION: i32 = 113;
const ID_LABEL_VERSION: i32 = 120;
const ID_LABEL_AUTHOR: i32 = 121;

/// Timer ID for countdown update
const TIMER_ID_COUNTDOWN: usize = 1;
const VK_ESCAPE_KEY: u32 = 0x1B;

/// Blink interval options (phút)
const BLINK_OPTIONS: [u32; 4] = [1, 5, 10, 30];
/// Stand-up interval options (phút)
const STANDUP_OPTIONS: [u32; 3] = [30, 45, 60];

/// Metadata từ Cargo
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// Global state cho dialog
static DIALOG_CONFIG: OnceLock<std::sync::Mutex<ConfigDialogState>> = OnceLock::new();
/// Global HWND (raw) để track config window singleton; dùng isize để tránh ràng buộc Send/Sync
static CONFIG_HWND: std::sync::Mutex<Option<isize>> = std::sync::Mutex::new(None);

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

/// Helper: lấy control handle, panic nếu thất bại (UI nội bộ)
fn dlg_item(hwnd: HWND, id: i32) -> HWND {
    unsafe { GetDlgItem(Some(hwnd), id).expect("GetDlgItem failed") }
}

/// Helper: gửi message với WPARAM/LPARAM bắt buộc
fn send_msg(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { SendMessageW(hwnd, msg, Some(wparam), Some(lparam)) }
}

/// Helper: gửi message không có tham số
/// Helper: set font cho control
fn set_font(hwnd: HWND, hfont: HGDIOBJ) {
    let _ = send_msg(hwnd, WM_SETFONT, WPARAM(hfont.0 as _), LPARAM(1));
}

/// Helper: set text cho control
fn set_text(hwnd: HWND, text_wide: &[u16]) {
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR::from_raw(text_wide.as_ptr()));
    }
}

/// Helper: set/get selection combobox
fn combo_set_cur_sel(hwnd: HWND, idx: usize) {
    let _ = send_msg(hwnd, CB_SETCURSEL, WPARAM(idx), LPARAM(0));
}

fn combo_get_cur_sel(hwnd: HWND) -> usize {
    send_msg(hwnd, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize
}

/// Helper: tạo control và unwrap HWND
fn create_control(
    ex_style: WINDOW_EX_STYLE,
    class: PCWSTR,
    title: PCWSTR,
    style: WINDOW_STYLE,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: HWND,
    menu: HMENU,
    hinstance: HINSTANCE,
) -> HWND {
    unsafe {
        CreateWindowExW(
            ex_style,
            class,
            title,
            style,
            x,
            y,
            width,
            height,
            Some(parent),
            Some(menu),
            Some(hinstance),
            None,
        )
        .expect("CreateWindowExW failed")
    }
}

/// Helper: timer wrappers
fn set_timer(hwnd: HWND, id: usize, interval_ms: u32) {
    unsafe {
        let _ = SetTimer(Some(hwnd), id, interval_ms, None);
    }
}

fn kill_timer(hwnd: HWND, id: usize) {
    unsafe {
        let _ = KillTimer(Some(hwnd), id);
    }
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
            // Lưu HWND vào global để track singleton
            if let Ok(mut guard) = CONFIG_HWND.lock() {
                *guard = Some(hwnd.0 as isize);
            }
            create_controls(hwnd);
            load_config_to_controls(hwnd);
            // Start countdown timer (1 second interval)
            set_timer(hwnd, TIMER_ID_COUNTDOWN, 1000);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_ID_COUNTDOWN {
                update_countdown_labels(hwnd);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 as u32 == VK_ESCAPE_KEY {
                kill_timer(hwnd, TIMER_ID_COUNTDOWN);
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as i32;
            let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;

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
                ID_BTN_CLOSE => {
                    log::info!("Close clicked");
                    kill_timer(hwnd, TIMER_ID_COUNTDOWN);
                    let _ = DestroyWindow(hwnd);
                }
                // Tự động lưu khi checkbox thay đổi
                ID_CHECKBOX_STARTUP => {
                    if notification == BN_CLICKED as u32 {
                        log::info!("Startup checkbox changed, auto-saving");
                        auto_save_config(hwnd);
                    }
                }
                // Tự động lưu khi combobox thay đổi
                ID_COMBO_BLINK | ID_COMBO_STANDUP | ID_COMBO_POSITION => {
                    if notification == CBN_SELCHANGE as u32 {
                        log::info!("Combobox changed, auto-saving");
                        auto_save_config(hwnd);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            kill_timer(hwnd, TIMER_ID_COUNTDOWN);
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            // Clear HWND khi window đóng
            if let Ok(mut guard) = CONFIG_HWND.lock() {
                *guard = None;
            }
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
    let checkbox = create_control(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Chạy khi Windows khởi động"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        20,
        20,
        250,
        25,
        hwnd,
        HMENU(ID_CHECKBOX_STARTUP as _),
        hinstance,
    );
    set_font(checkbox, hfont);

    // === BLINK SECTION ===
    // Label: Blink interval + countdown + test button trên cùng một dòng
    let label_blink = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("Nhắc chớp mắt (phút):"),
        WS_CHILD | WS_VISIBLE,
        20,
        60,
        130,
        24,
        hwnd,
        HMENU(ID_LABEL_BLINK as _),
        hinstance,
    );
    set_font(label_blink, hfont);

    // Combo: Blink interval
    let combo_blink = create_control(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        155,
        55,
        55,
        120,
        hwnd,
        HMENU(ID_COMBO_BLINK as _),
        hinstance,
    );
    set_font(combo_blink, hfont);

    // Add blink options
    for opt in BLINK_OPTIONS {
        let text: Vec<u16> = format!("{}", opt)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        send_msg(combo_blink, CB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as _));
    }

    // Label: Blink countdown
    let label_blink_countdown = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("⏱️ --:--"),
        WS_CHILD | WS_VISIBLE,
        220,
        60,
        60,
        24,
        hwnd,
        HMENU(ID_LABEL_BLINK_COUNTDOWN as _),
        hinstance,
    );
    set_font(label_blink_countdown, hfont);

    // Button: Test Blink (trên cùng dòng)
    let btn_test_blink = create_control(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Test"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        290,
        55,
        50,
        24,
        hwnd,
        HMENU(ID_BTN_TEST_BLINK as _),
        hinstance,
    );
    set_font(btn_test_blink, hfont);

    // === STANDUP SECTION ===
    // Label: Stand-up interval + countdown + test button
    let label_standup = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("Nhắc đứng dậy (phút):"),
        WS_CHILD | WS_VISIBLE,
        20,
        95,
        130,
        24,
        hwnd,
        HMENU(ID_LABEL_STANDUP as _),
        hinstance,
    );
    set_font(label_standup, hfont);

    // Combo: Stand-up interval
    let combo_standup = create_control(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        155,
        90,
        55,
        120,
        hwnd,
        HMENU(ID_COMBO_STANDUP as _),
        hinstance,
    );
    set_font(combo_standup, hfont);

    // Add stand-up options
    for opt in STANDUP_OPTIONS {
        let text: Vec<u16> = format!("{}", opt)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        send_msg(combo_standup, CB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as _));
    }

    // Label: Standup countdown
    let label_standup_countdown = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("⏱️ --:--"),
        WS_CHILD | WS_VISIBLE,
        220,
        95,
        60,
        24,
        hwnd,
        HMENU(ID_LABEL_STANDUP_COUNTDOWN as _),
        hinstance,
    );
    set_font(label_standup_countdown, hfont);

    // Button: Test Stand Up (trên cùng dòng)
    let btn_test_standup = create_control(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Test"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        290,
        90,
        50,
        24,
        hwnd,
        HMENU(ID_BTN_TEST_STANDUP as _),
        hinstance,
    );
    set_font(btn_test_standup, hfont);

    // === POSITION SECTION ===
    // Label: Position
    let label_position = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        w!("Vị trí thông báo:"),
        WS_CHILD | WS_VISIBLE,
        20,
        130,
        120,
        20,
        hwnd,
        HMENU(ID_LABEL_POSITION as _),
        hinstance,
    );
    set_font(label_position, hfont);

    // Combo: Position
    let combo_position = create_control(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        145,
        125,
        140,
        150,
        hwnd,
        HMENU(ID_COMBO_POSITION as _),
        hinstance,
    );
    set_font(combo_position, hfont);

    // Add position options
    for pos in crate::config::OverlayPosition::all() {
        let text: Vec<u16> = pos
            .display_name()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        send_msg(combo_position, CB_ADDSTRING, WPARAM(0), LPARAM(text.as_ptr() as _));
    }

    // === BUTTONS ===
    // Version & author labels
    let version_label = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE,
        20,
        160,
        270,
        18,
        hwnd,
        HMENU(ID_LABEL_VERSION as _),
        hinstance,
    );
    let version_text: Vec<u16> = format!("Phiên bản: {}", APP_VERSION)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    set_text(version_label, &version_text);
    set_font(version_label, hfont);

    let author_label = create_control(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE,
        20,
        180,
        270,
        18,
        hwnd,
        HMENU(ID_LABEL_AUTHOR as _),
        hinstance,
    );
    let author_text: Vec<u16> = format!("Tác giả: {}", APP_AUTHORS)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    set_text(author_label, &author_text);
    set_font(author_label, hfont);
}

/// Update countdown labels
unsafe fn update_countdown_labels(hwnd: HWND) {
    if let Some(info) = timer::get_timer_info() {
        // Update blink countdown
        let blink_secs = info.blink_remaining_secs();
        let blink_text = format!("⏱️ {}", format_countdown(blink_secs));
        let blink_wide: Vec<u16> = blink_text
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let blink_label = dlg_item(hwnd, ID_LABEL_BLINK_COUNTDOWN);
        set_text(blink_label, &blink_wide);

        // Update standup countdown
        let standup_secs = info.standup_remaining_secs();
        let standup_text = format!("⏱️ {}", format_countdown(standup_secs));
        let standup_wide: Vec<u16> = standup_text
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let standup_label = dlg_item(hwnd, ID_LABEL_STANDUP_COUNTDOWN);
        set_text(standup_label, &standup_wide);
    }
}

/// Load config values vào controls
unsafe fn load_config_to_controls(hwnd: HWND) {
    let state = DIALOG_CONFIG.get().and_then(|m| m.lock().ok());
    if let Some(state) = state {
        let config = &state.config;

        // Checkbox startup
        let checkbox = dlg_item(hwnd, ID_CHECKBOX_STARTUP);
        let flag = if config.startup {
            BST_CHECKED.0 as _
        } else {
            BST_UNCHECKED.0 as _
        };
        send_msg(checkbox, BM_SETCHECK, WPARAM(flag), LPARAM(0));

        // Combo blink interval
        let combo_blink = dlg_item(hwnd, ID_COMBO_BLINK);
        let blink_idx = BLINK_OPTIONS
            .iter()
            .position(|&x| x == config.blink_interval)
            .unwrap_or(1);
        combo_set_cur_sel(combo_blink, blink_idx);

        // Combo stand-up interval
        let combo_standup = dlg_item(hwnd, ID_COMBO_STANDUP);
        let standup_idx = STANDUP_OPTIONS
            .iter()
            .position(|&x| x == config.standup_interval)
            .unwrap_or(1);
        combo_set_cur_sel(combo_standup, standup_idx);

        // Combo position
        let combo_position = dlg_item(hwnd, ID_COMBO_POSITION);
        let position_idx = config.overlay_position.index();
        combo_set_cur_sel(combo_position, position_idx);
    }

    // Initial countdown update
    update_countdown_labels(hwnd);
}

/// Đọc config từ controls
unsafe fn read_config_from_controls(hwnd: HWND) -> AppConfig {
    // Đọc values từ controls
    let checkbox = dlg_item(hwnd, ID_CHECKBOX_STARTUP);
    let startup = send_msg(checkbox, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 == BST_CHECKED.0 as isize;

    let combo_blink = dlg_item(hwnd, ID_COMBO_BLINK);
    let blink_idx = combo_get_cur_sel(combo_blink);
    let blink_interval = BLINK_OPTIONS.get(blink_idx).copied().unwrap_or(5);

    let combo_standup = dlg_item(hwnd, ID_COMBO_STANDUP);
    let standup_idx = combo_get_cur_sel(combo_standup);
    let standup_interval = STANDUP_OPTIONS.get(standup_idx).copied().unwrap_or(45);

    let combo_position = dlg_item(hwnd, ID_COMBO_POSITION);
    let position_idx = combo_get_cur_sel(combo_position);
    let overlay_position = crate::config::OverlayPosition::from_index(position_idx);

    AppConfig {
        startup,
        blink_interval,
        standup_interval,
        overlay_position,
    }
}

/// Tự động lưu config khi thay đổi (không hiện thông báo)
unsafe fn auto_save_config(hwnd: HWND) {
    let new_config = read_config_from_controls(hwnd);

    log::info!("Auto-saving config: {:?}", new_config);

    // Lưu vào file
    if let Err(e) = crate::config::save_config(&new_config) {
        log::error!("Failed to save config: {}", e);
        return;
    }

    // Update registry nếu startup thay đổi
    if let Err(e) = registry::set_startup(new_config.startup) {
        log::error!("Failed to update registry: {}", e);
    }

    // Gửi config mới đến main thread để update timer
    if let Some(state) = DIALOG_CONFIG.get().and_then(|m| m.lock().ok()) {
        if let Some(tx) = &state.config_tx {
            let _ = tx.try_send(Arc::new(new_config.clone()));
        }
    }

    // Update state
    if let Some(mut state) = DIALOG_CONFIG.get().and_then(|m| m.lock().ok()) {
        state.config = new_config;
    }
}

/// Hiển thị config dialog
pub fn show_config_dialog(config: Arc<AppConfig>, config_tx: Option<mpsc::Sender<Arc<AppConfig>>>) {
    // Kiểm tra xem đã có config window đang mở chưa
    if let Ok(guard) = CONFIG_HWND.lock() {
        if let Some(existing_hwnd) = *guard {
            let existing_hwnd = HWND(existing_hwnd as _);
            // Đã có window, đưa lên foreground
            unsafe {
                if IsWindow(Some(existing_hwnd)).as_bool() {
                    log::info!("Config window đã mở, đưa lên foreground");
                    let _ = ShowWindow(existing_hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(existing_hwnd);
                    return;
                }
            }
        }
    }

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
                hCursor: LoadCursorW(Some(HINSTANCE::default()), IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as _),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
            };

            RegisterClassW(&wc);

            // Tính vị trí center screen
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            let dialog_width = 360;
            let dialog_height = 250; // vừa đủ cho controls + metadata
            let x = (screen_width - dialog_width) / 2;
            let y = (screen_height - dialog_height) / 2;

            // Tạo window
            let hwnd = CreateWindowExW(
                WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
                class_name,
                w!("Blink Reminder - Cấu hình"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                x,
                y,
                dialog_width,
                dialog_height,
                Some(HWND::default()),
                Some(HMENU::default()),
                Some(HINSTANCE::default()),
                None,
            );

            let Ok(hwnd) = hwnd else {
                log::error!("Failed to create config dialog");
                return;
            };

            // Show window
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);

            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if !IsDialogMessageW(hwnd, &msg).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    });
}
