use crate::config::OverlayPosition;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::CoInitializeEx;
use windows::Win32::System::Com::COINIT_APARTMENTTHREADED;
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

/// Kiểm tra xem overlay có đang hiển thị không
pub fn is_overlay_active() -> bool {
    if let Some(mutex) = ANIM_STATE.get() {
        if let Ok(guard) = mutex.lock() {
            if let Some(state) = guard.as_ref() {
                return state.active.load(Ordering::SeqCst);
            }
        }
    }
    false
}

/// Hiển thị overlay animation
pub fn show_overlay(overlay_type: OverlayType) -> Result<()> {
    // Kiểm tra nếu đã có overlay đang hiển thị, bỏ qua
    if is_overlay_active() {
        log::info!("Bỏ qua overlay {:?} vì đã có overlay đang hiển thị", overlay_type);
        return Ok(());
    }

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

/// Tạo overlay window với Direct2D rendering
fn create_overlay_window(state: Arc<AnimationState>) -> Result<()> {
    unsafe {
        // Khởi tạo COM cho Direct2D/DirectWrite
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

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
            OverlayPosition::BottomLeft => (margin, screen_height - window_height - margin - 80),
            OverlayPosition::BottomRight => (
                screen_width - window_width - margin,
                screen_height - window_height - margin - 80,
            ),
        };

        // Tạo window với WS_EX_LAYERED cho transparency
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
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

        // Sử dụng UpdateLayeredWindow với per-pixel alpha thay vì color key
        // Trước tiên cần tạo DIB section với alpha channel
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

        // Tạo BITMAPINFO cho 32-bit ARGB bitmap
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: window_width,
                biHeight: -window_height, // Top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(
            Some(hdc_mem),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?;

        let old_bmp = SelectObject(hdc_mem, hbmp.into());

        // Tạo Direct2D render target cho DC
        let d2d_factory: ID2D1Factory1 = D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            None,
        )?;

        let render_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 0.0,
            dpiY: 0.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        let dc_render_target = d2d_factory.CreateDCRenderTarget(&render_props)?;

        let rect = RECT {
            left: 0,
            top: 0,
            right: window_width,
            bottom: window_height,
        };
        dc_render_target.BindDC(hdc_mem, &rect)?;

        // Tạo DirectWrite factory và text format
        let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

        let font_name: Vec<u16> = "Segoe UI Emoji\0".encode_utf16().collect();
        let locale: Vec<u16> = "en-US\0".encode_utf16().collect();

        let text_format = dwrite_factory.CreateTextFormat(
            PCWSTR::from_raw(font_name.as_ptr()),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            200.0,
            PCWSTR::from_raw(locale.as_ptr()),
        )?;

        text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;

        // Tạo brush cho text
        let text_brush = dc_render_target.CreateSolidColorBrush(
            &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            None,
        )?;

        // Hiển thị window
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        // Hàm render một frame
        let render_frame = |frame_index: u32| -> Result<()> {
            let frames = get_animation_frames(state.overlay_type);
            let emoji = frames.get(frame_index as usize).unwrap_or(&frames[0]);
            let emoji_wide: Vec<u16> = emoji.encode_utf16().collect();

            dc_render_target.BeginDraw();

            // Clear với transparent
            dc_render_target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            let size = dc_render_target.GetSize();
            let layout_rect = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: size.width,
                bottom: size.height,
            };

            // Vẽ emoji với color font support
            dc_render_target.DrawText(
                &emoji_wide,
                &text_format,
                &layout_rect,
                &text_brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            dc_render_target.EndDraw(None, None)?;

            // Update layered window với per-pixel alpha
            let blend = BLENDFUNCTION {
                BlendOp: 0, // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
            };

            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x, y };
            let size_wnd = SIZE {
                cx: window_width,
                cy: window_height,
            };

            UpdateLayeredWindow(
                hwnd,
                Some(hdc_screen),
                Some(&pt_dst),
                Some(&size_wnd),
                Some(hdc_mem),
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )?;

            Ok(())
        };

        // Render frame đầu tiên
        render_frame(0)?;

        // Set timer để update animation
        SetTimer(Some(hwnd), 1, 100, None);

        // Message loop với animation
        let mut msg = MSG::default();
        let mut last_frame = 0u32;

        while state.active.load(Ordering::SeqCst) {
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
                if let Err(e) = render_frame(current_frame) {
                    log::warn!("Render error: {:?}", e);
                }
                last_frame = current_frame;
            }

            std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }

        // Cleanup
        let _ = KillTimer(Some(hwnd), 1);
        SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(PCWSTR::from_raw(class_name.as_ptr()), Some(HINSTANCE::default()));

        log::info!("Overlay window closed");
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
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
