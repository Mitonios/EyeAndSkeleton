use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;
use std::time::Duration;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Loại overlay để hiển thị
#[derive(Debug, Clone, Copy)]
pub enum OverlayType {
    Blink,
    StandUp,
}

/// Hiển thị overlay animation
pub fn show_overlay(overlay_type: OverlayType) -> Result<()> {
    log::info!("Hiển thị overlay: {:?}", overlay_type);

    // Tạo shared state để quản lý window lifecycle
    let window_active = Arc::new(AtomicBool::new(true));

    // Spawn thread để tạo và quản lý window
    let window_active_clone = window_active.clone();
    std::thread::spawn(move || {
        if let Err(e) = create_overlay_window(overlay_type, window_active_clone) {
            log::error!("Lỗi khi tạo overlay window: {}", e);
        }
    });

    // Đợi 4 giây rồi tắt
    std::thread::sleep(Duration::from_secs(4));
    window_active.store(false, Ordering::SeqCst);

    Ok(())
}

/// Tạo overlay window
fn create_overlay_window(overlay_type: OverlayType, active: Arc<AtomicBool>) -> Result<()> {
    unsafe {
        // Đăng ký window class
        let class_name_wide: Vec<u16> = "BlinkReminderOverlayClass\0".encode_utf16().collect();
        let class_name = PCWSTR::from_raw(class_name_wide.as_ptr());
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE::default(),
            hIcon: HICON::default(),
            hCursor: HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
        };

        if RegisterClassW(&wc) == 0 {
            return Err(anyhow::anyhow!("Không thể đăng ký window class"));
        }

        // Lấy thông tin màn hình
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let _screen_height = GetSystemMetrics(SM_CYSCREEN);

        // Vị trí góc phải trên
        let window_width = 200;
        let window_height = 200;
        let x = screen_width - window_width - 20; // Margin 20px
        let y = 20;

        // Tạo window
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT,
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            x, y, window_width, window_height,
            HWND::default(),
            HMENU::default(),
            HINSTANCE::default(),
            None,
        );

        if hwnd == HWND::default() {
            return Err(anyhow::anyhow!("Không thể tạo overlay window"));
        }

        // Làm window transparent
        if let Err(e) = SetLayeredWindowAttributes(hwnd, COLORREF(0), 240, LWA_ALPHA) {
            log::warn!("Không thể set layered attributes: {:?}", e);
        }

        // Hiển thị window
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        UpdateWindow(hwnd);

        // Vẽ emoji ban đầu
        draw_emoji(hwnd, overlay_type)?;

        // Message loop
        let mut msg = MSG::default();
        while active.load(Ordering::SeqCst) && GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(PCWSTR::from_raw(class_name.as_ptr()), HINSTANCE::default());

        log::info!("Overlay window closed");
        Ok(())
    }
}

/// Vẽ emoji lên window
fn draw_emoji(hwnd: HWND, overlay_type: OverlayType) -> Result<()> {
    unsafe {
        let hdc = GetDC(hwnd);
        if hdc.is_invalid() {
            return Err(anyhow::anyhow!("Không thể lấy device context"));
        }

        // Set background mode to transparent
        SetBkMode(hdc, TRANSPARENT);

        // Set text color to white for visibility
        SetTextColor(hdc, COLORREF(0x00FFFFFF));

        // Tạo font lớn cho emoji
        let font_name_wide: Vec<u16> = "Segoe UI Emoji\0".encode_utf16().collect();
        let font = CreateFontW(
            120, 0, 0, 0, 400, 0, 0, 0,  // FW_NORMAL = 400
            0, 0, 0,  // DEFAULT_CHARSET = 0, OUT_DEFAULT_PRECIS = 0, CLIP_DEFAULT_PRECIS = 0
            0, 0, PCWSTR::from_raw(font_name_wide.as_ptr()),  // DEFAULT_QUALITY = 0, DEFAULT_PITCH = 0
        );

        if font.is_invalid() {
            let _ = ReleaseDC(hwnd, hdc);
            return Err(anyhow::anyhow!("Không thể tạo font"));
        }

        // Select font
        let old_font = SelectObject(hdc, font);

        // Chọn emoji
        let emoji = match overlay_type {
            OverlayType::Blink => "👁️",
            OverlayType::StandUp => "🧍",
        };

        // Convert emoji to wide string (without null terminator for TextOutW)
        let emoji_wide: Vec<u16> = emoji.encode_utf16().collect();

        // Vẽ emoji ở center
        TextOutW(hdc, 50, 50, &emoji_wide);

        // Cleanup
        SelectObject(hdc, old_font);
        let _ = DeleteObject(font);
        let _ = ReleaseDC(hwnd, hdc);

        Ok(())
    }
}

/// Window procedure cho overlay window
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            // Repaint if needed
            let mut ps = PAINTSTRUCT::default();
            let _hdc = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}