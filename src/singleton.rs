//! Singleton pattern cho Blink Reminder
//!
//! Đảm bảo chỉ có 1 instance chạy tại 1 thời điểm.
//! Nếu app đã chạy, gửi signal để mở Config window.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::{CreateMutexW, OpenMutexW, MUTEX_ALL_ACCESS};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Mutex name cho singleton check
const MUTEX_NAME: PCWSTR = w!("BlinkReminderSingletonMutex");

/// Window class name cho IPC
pub const IPC_WINDOW_CLASS: PCWSTR = w!("BlinkReminderIPCWindow");

/// Custom message để yêu cầu mở Config window
pub const WM_SHOW_CONFIG: u32 = WM_USER + 200;

/// Singleton guard - giữ mutex handle
pub struct SingletonGuard {
    #[allow(dead_code)]
    mutex_handle: HANDLE,
}

impl Drop for SingletonGuard {
    fn drop(&mut self) {
        unsafe {
            if self.mutex_handle != HANDLE::default() {
                let _ = CloseHandle(self.mutex_handle);
            }
        }
    }
}

/// Kiểm tra và acquire singleton
pub fn acquire_singleton() -> anyhow::Result<Option<SingletonGuard>> {
    unsafe {
        // Thử mở mutex đã tồn tại
        let existing = OpenMutexW(MUTEX_ALL_ACCESS, false, MUTEX_NAME);

        if let Ok(handle) = existing {
            // Mutex đã tồn tại = app đã chạy
            let _ = CloseHandle(handle);

            // Gửi signal để mở Config window
            signal_existing_instance();

            return Ok(None);
        }

        // Tạo mutex mới
        let handle = CreateMutexW(None, true, MUTEX_NAME)?;

        // Đây là instance đầu tiên
        Ok(Some(SingletonGuard {
            mutex_handle: handle,
        }))
    }
}

/// Gửi signal đến instance đang chạy để mở Config window
fn signal_existing_instance() {
    log::info!("Đã có instance đang chạy, gửi signal mở Config window");

    unsafe {
        // Tìm IPC window của instance đang chạy
        match FindWindowW(IPC_WINDOW_CLASS, PCWSTR::null()) {
            Ok(hwnd) if hwnd != HWND::default() => {
                let _ = PostMessageW(Some(hwnd), WM_SHOW_CONFIG, WPARAM(0), LPARAM(0));
                log::info!("Đã gửi WM_SHOW_CONFIG đến instance đang chạy");
            }
            Ok(_) => {
                log::warn!("Không tìm thấy IPC window");
            }
            Err(err) => {
                log::warn!("FindWindowW lỗi: {:?}", err);
            }
        }
    }
}

/// Global callback cho IPC
static IPC_CALLBACK: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

/// Tạo IPC window để nhận signals từ instances khác
pub fn create_ipc_window(on_show_config: impl Fn() + Send + Sync + 'static) -> anyhow::Result<()> {
    // Lưu callback vào global
    let _ = IPC_CALLBACK.set(Box::new(on_show_config));

    std::thread::spawn(move || {
        unsafe {
            // Đăng ký window class
            let wc = WNDCLASSW {
                style: WNDCLASS_STYLES(0),
                lpfnWndProc: Some(ipc_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: HINSTANCE::default(),
                hIcon: HICON::default(),
                hCursor: HCURSOR::default(),
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: IPC_WINDOW_CLASS,
            };

            RegisterClassW(&wc);

            // Tạo hidden window
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                IPC_WINDOW_CLASS,
                PCWSTR::null(),
                WS_OVERLAPPED,
                0,
                0,
                1,
                1,
                Some(HWND::default()),
                Some(HMENU::default()),
                Some(HINSTANCE::default()),
                None,
            );

            let Ok(_hwnd) = hwnd else {
                log::error!("Không thể tạo IPC window");
                return;
            };

            log::info!("Đã tạo IPC window");

            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    Ok(())
}

/// Window procedure cho IPC window
unsafe extern "system" fn ipc_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => LRESULT(0),
        WM_SHOW_CONFIG => {
            log::info!("Nhận WM_SHOW_CONFIG từ instance khác");
            // Gọi global callback
            if let Some(callback) = IPC_CALLBACK.get() {
                callback();
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
