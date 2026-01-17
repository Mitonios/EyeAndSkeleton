use crate::config::OverlayPosition;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Loại overlay để hiển thị
#[derive(Debug, Clone, Copy)]
pub enum OverlayType {
    Blink,
    StandUp,
}

/// Thời gian hiển thị overlay (5 giây)
const OVERLAY_DURATION_SECS: u64 = 5;

/// Thời gian mỗi frame (ms)
const FRAME_DURATION_MS: u64 = 400;

/// Animation frames cho mỗi loại
fn get_animation_frames(overlay_type: OverlayType) -> Vec<&'static str> {
    match overlay_type {
        OverlayType::Blink => vec!["😌", "🙂"],   // Nhắm - Mở
        OverlayType::StandUp => vec!["🧍", "🧎"], // Đứng - Quỳ
    }
}

/// Lấy số frame của animation
fn get_animation_frame_count(overlay_type: OverlayType) -> u32 {
    get_animation_frames(overlay_type).len() as u32
}

/// Shared state cho animation
struct AnimationState {
    active: AtomicBool,
    current_frame: AtomicU32,
    overlay_type: OverlayType,
    position: OverlayPosition,
}

/// Global animation state (thread-safe với OnceLock + Mutex)
static ANIM_STATE: OnceLock<Mutex<Option<Arc<AnimationState>>>> = OnceLock::new();

/// Hiển thị overlay animation
pub fn show_overlay(overlay_type: OverlayType) -> Result<()> {
    // Đọc vị trí từ config
    let position = crate::config::load_config()
        .map(|c| c.overlay_position)
        .unwrap_or_default();

    log::info!("Hiển thị overlay: {:?} tại {:?}", overlay_type, position);

    // Tạo shared state
    let state = Arc::new(AnimationState {
        active: AtomicBool::new(true),
        current_frame: AtomicU32::new(0),
        overlay_type,
        position,
    });

    // Store state an toàn với OnceLock + Mutex
    let _ = ANIM_STATE.get_or_init(|| Mutex::new(None));
    if let Some(mutex) = ANIM_STATE.get() {
        if let Ok(mut guard) = mutex.lock() {
            *guard = Some(state.clone());
        }
    }

    // Clone cho thread
    let state_clone = state.clone();

    // Spawn thread để tạo và quản lý window
    std::thread::spawn(move || {
        if let Err(e) = create_overlay_window(state_clone) {
            log::error!("Lỗi khi tạo overlay window: {}", e);
        }
    });

    // Thread để update animation frames
    let state_anim = state.clone();
    std::thread::spawn(move || {
        let start = Instant::now();
        let mut last_frame_time = start;

        while state_anim.active.load(Ordering::SeqCst) {
            // Check timeout
            if start.elapsed() >= Duration::from_secs(OVERLAY_DURATION_SECS) {
                state_anim.active.store(false, Ordering::SeqCst);
                break;
            }

            // Update frame
            if last_frame_time.elapsed() >= Duration::from_millis(FRAME_DURATION_MS) {
                let current = state_anim.current_frame.load(Ordering::SeqCst);
                let frame_count = get_animation_frame_count(state_anim.overlay_type);
                let next = (current + 1) % frame_count;
                state_anim.current_frame.store(next, Ordering::SeqCst);
                last_frame_time = Instant::now();
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    });

    // Đợi 5 giây rồi cleanup
    std::thread::sleep(Duration::from_secs(OVERLAY_DURATION_SECS));
    state.active.store(false, Ordering::SeqCst);

    // Clear global state an toàn
    if let Some(mutex) = ANIM_STATE.get() {
        if let Ok(mut guard) = mutex.lock() {
            *guard = None;
        }
    }

    Ok(())
}

/// Tạo overlay window
fn create_overlay_window(state: Arc<AnimationState>) -> Result<()> {
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

        RegisterClassW(&wc);

        // Lấy thông tin màn hình
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        // Kích thước window
        let window_width = 400;
        let window_height = 400;
        let margin = 40;

        // Tính vị trí dựa trên position
        let (x, y) = match state.position {
            OverlayPosition::TopLeft => (margin, margin),
            OverlayPosition::TopRight => (screen_width - window_width - margin, margin),
            OverlayPosition::Center => (
                (screen_width - window_width) / 2,
                (screen_height - window_height) / 2,
            ),
            OverlayPosition::BottomLeft => (margin, screen_height - window_height - margin - 80), // -80 cho taskbar
            OverlayPosition::BottomRight => (
                screen_width - window_width - margin,
                screen_height - window_height - margin - 80,
            ),
        };

        // Tạo window
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            x,
            y,
            window_width,
            window_height,
            Some(HWND::default()),
            Some(HMENU::default()),
            Some(HINSTANCE::default()),
            None,
        )?;

        // Làm window transparent - dùng màu trắng làm color key
        if let Err(e) = SetLayeredWindowAttributes(hwnd, COLORREF(0x00FFFFFF), 0, LWA_COLORKEY) {
            log::warn!("Không thể set layered attributes: {:?}", e);
        }

        // Hiển thị window
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(hwnd);

        // Vẽ emoji ban đầu
        draw_emoji(hwnd, state.overlay_type, 0)?;

        // Set timer để update animation (100ms)
        SetTimer(Some(hwnd), 1, 100, None);

        // Message loop với animation
        let mut msg = MSG::default();
        let mut last_frame = 0u32;

        while state.active.load(Ordering::SeqCst) {
            // Check for messages với timeout
            if PeekMessageW(&mut msg, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Update animation
            let current_frame = state.current_frame.load(Ordering::SeqCst);
            if current_frame != last_frame {
                draw_emoji(hwnd, state.overlay_type, current_frame)?;
                last_frame = current_frame;
            }

            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }

        // Cleanup
        let _ = KillTimer(Some(hwnd), 1);
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(PCWSTR::from_raw(class_name.as_ptr()), Some(HINSTANCE::default()));

        log::info!("Overlay window closed");
        Ok(())
    }
}

/// Vẽ emoji lên window với frame index
fn draw_emoji(hwnd: HWND, overlay_type: OverlayType, frame_index: u32) -> Result<()> {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        if hdc.is_invalid() {
            return Err(anyhow::anyhow!("Không thể lấy device context"));
        }

        // Fill background với màu trắng (sẽ trở thành transparent qua color key)
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let brush = CreateSolidBrush(COLORREF(0x00FFFFFF)); // White = transparent
        FillRect(hdc, &rect, brush);
        let _ = DeleteObject(brush.into());

        // Set background mode to transparent for text
        SetBkMode(hdc, TRANSPARENT);

        // Tạo font lớn cho emoji
        let font_name_wide: Vec<u16> = "Segoe UI Emoji\0".encode_utf16().collect();
        let font = CreateFontW(
            240,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            FONT_CHARSET(0),
            FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0),
            FONT_QUALITY(0),
            0,
            PCWSTR::from_raw(font_name_wide.as_ptr()),
        );

        if font.is_invalid() {
            let _ = ReleaseDC(Some(hwnd), hdc);
            return Err(anyhow::anyhow!("Không thể tạo font"));
        }

        // Select font
        let old_font = SelectObject(hdc, font.into());

        // Lấy emoji frame
        let frames = get_animation_frames(overlay_type);
        let emoji = frames.get(frame_index as usize).unwrap_or(&frames[0]);

        // Convert emoji to wide string
        let emoji_wide: Vec<u16> = emoji.encode_utf16().collect();

        // Vẽ emoji ở center
        let _ = TextOutW(hdc, 80, 80, &emoji_wide);

        // Cleanup
        SelectObject(hdc, old_font);
        let _ = DeleteObject(font.into());
        let _ = ReleaseDC(Some(hwnd), hdc);

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
            let mut ps = PAINTSTRUCT::default();
            let _hdc = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_TIMER => {
            // Trigger redraw
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
