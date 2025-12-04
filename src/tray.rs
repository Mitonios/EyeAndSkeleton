use tokio::sync::mpsc;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::*;
use windows::core::{PCWSTR, w};
use std::sync::OnceLock;
use anyhow::Result;

/// Events sent from tray to main thread
#[derive(Debug)]
pub enum TrayEvent {
    ShowConfig,
    Exit,
}

/// Global sender để window procedure có thể gửi events
static TRAY_SENDER: OnceLock<mpsc::Sender<TrayEvent>> = OnceLock::new();

/// Custom window message for tray icon
const WM_TRAYICON: u32 = WM_USER + 100;

/// Struct để quản lý tray icon
pub struct TrayManager {
    #[allow(dead_code)]
    nid: NOTIFYICONDATAW,
    message_window: HWND,
}

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
            log::info!("WM_TRAYICON received: mouse_msg=0x{:X}", mouse_msg);
            
            match mouse_msg {
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    log::info!("Right-click detected, showing context menu");
                    show_context_menu(hwnd);
                }
                WM_LBUTTONUP => {
                    log::info!("Left-click detected, opening config window");
                    // Gửi event để mở Config window
                    if let Some(tx) = TRAY_SENDER.get() {
                        let _ = tx.try_send(TrayEvent::ShowConfig);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let menu_id = (wparam.0 & 0xFFFF) as u32;
            log::info!("WM_COMMAND received: menu_id={}", menu_id);
            
            if let Some(tx) = TRAY_SENDER.get() {
                match menu_id {
                    1 => {
                        log::info!("Config menu selected");
                        let _ = tx.try_send(TrayEvent::ShowConfig);
                    }
                    2 => {
                        log::info!("Exit menu selected");
                        let _ = tx.try_send(TrayEvent::Exit);
                    }
                    _ => {}
                }
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

/// Hiển thị context menu thực sự
unsafe fn show_context_menu(hwnd: HWND) {
    // Lấy vị trí cursor
    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);
    
    // Tạo popup menu
    let hmenu = CreatePopupMenu().unwrap_or_default();
    if hmenu.is_invalid() {
        log::error!("Failed to create popup menu");
        return;
    }
    
    // Thêm menu items
    let _ = AppendMenuW(hmenu, MF_STRING, 1, w!("Config..."));
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(hmenu, MF_STRING, 2, w!("Exit"));
    
    // Set foreground window để menu đóng khi click ra ngoài
    let _ = SetForegroundWindow(hwnd);
    
    // Track popup menu
    let _ = TrackPopupMenu(
        hmenu,
        TPM_BOTTOMALIGN | TPM_LEFTALIGN,
        point.x,
        point.y,
        0,
        hwnd,
        None,
    );
    
    // Cleanup
    let _ = DestroyMenu(hmenu);
}

impl TrayManager {
    /// Tạo tray manager mới - TẤT CẢ phải trên cùng một thread!
    pub fn new(tx: mpsc::Sender<TrayEvent>) -> Result<Self> {
        // Store sender globally cho window procedure
        let _ = TRAY_SENDER.set(tx.clone());
        
        // Channel để nhận kết quả từ tray thread
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        
        // Tạo window VÀ chạy message loop trên CÙNG MỘT thread
        std::thread::spawn(move || {
            log::info!("Tray thread started");
            
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
            
            let atom = unsafe { RegisterClassW(&wc) };
            if atom == 0 {
                log::warn!("RegisterClassW failed or class already registered");
            }
            
            // Tạo hidden window
            let message_window = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class_name,
                    w!("BlinkReminderTray"),
                    WS_OVERLAPPED,
                    0, 0, 1, 1,
                    HWND::default(),
                    HMENU::default(),
                    HINSTANCE::default(),
                    None,
                )
            };
            
            if message_window == HWND::default() {
                let _ = result_tx.send(Err(anyhow::anyhow!("Không thể tạo message window")));
                return;
            }
            log::info!("Created message window: {:?}", message_window);
            
            // Setup NOTIFYICONDATAW
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = message_window;
            nid.uID = 1;
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            
            // Load icon
            let hicon = load_icon();
            nid.hIcon = HICON(hicon.0);
            
            // Set tooltip
            let tooltip_text = "Blink Reminder";
            let tooltip_utf16: Vec<u16> = tooltip_text.encode_utf16().chain(std::iter::once(0)).collect();
            let copy_len = tooltip_utf16.len().min(128);
            nid.szTip[..copy_len].copy_from_slice(&tooltip_utf16[..copy_len]);
            
            log::info!("Adding tray icon...");
            
            // Add to tray
            let result = unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
            if !result.as_bool() {
                log::error!("Shell_NotifyIconW failed!");
                let _ = result_tx.send(Err(anyhow::anyhow!("Không thể thêm tray icon")));
                return;
            }
            log::info!("Shell_NotifyIconW succeeded!");
            
            // Gửi success
            let _ = result_tx.send(Ok((nid, message_window)));
            
            // Message loop - PHẢI chạy trên thread này
            log::info!("Starting message loop on tray thread");
            unsafe {
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    log::debug!("Message received: msg={}, hwnd={:?}", msg.message, msg.hwnd);
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            log::info!("Message loop ended");
            
            // Cleanup
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &nid);
                let _ = DestroyWindow(message_window);
            }
        });
        
        // Đợi kết quả từ tray thread
        match result_rx.recv() {
            Ok(Ok((nid, message_window))) => {
                log::info!("Tray icon created successfully");
                Ok(Self { nid, message_window })
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow::anyhow!("Tray thread crashed")),
        }
    }
}

impl Drop for TrayManager {
    fn drop(&mut self) {
        // Cleanup được xử lý trong tray thread khi message loop kết thúc
        // Gửi WM_QUIT để kết thúc message loop
        log::info!("TrayManager dropped - sending quit message");
        unsafe {
            if self.message_window != HWND::default() {
                PostMessageW(self.message_window, WM_QUIT, WPARAM(0), LPARAM(0)).ok();
            }
        }
    }
}

/// Khởi tạo system tray icon
pub fn init_tray(tx: mpsc::Sender<TrayEvent>) -> Result<TrayManager> {
    TrayManager::new(tx)
}

/// Load icon từ file hoặc dùng system icon
fn load_icon() -> HANDLE {
    // Thử load từ file
    if let Some(icon_path) = get_icon_path() {
        if let Ok(handle) = unsafe {
            LoadImageW(
                HINSTANCE::default(),
                PCWSTR::from_raw(icon_path.as_ptr()),
                IMAGE_ICON,
                16, 16,
                LR_LOADFROMFILE,
            )
        } {
            log::info!("Loaded icon from file");
            return handle;
        }
    }
    
    // Fallback to system icon
    log::warn!("Using system default icon");
    let hicon = unsafe { LoadIconW(HINSTANCE::default(), IDI_APPLICATION) };
    HANDLE(hicon.unwrap_or_default().0)
}

/// Lấy đường dẫn đến icon file
fn get_icon_path() -> Option<Vec<u16>> {
    // Thử tìm icon cạnh executable trước
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let icon_path = exe_dir.join("icon.ico");
            if icon_path.exists() {
                let path_str = icon_path.to_string_lossy();
                let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
                log::info!("Found icon at: {}", path_str);
                return Some(wide_path);
            }
        }
    }
    
    // Fallback: tìm trong current directory
    if let Ok(current_dir) = std::env::current_dir() {
        let icon_path = current_dir.join("icon.ico");
        if icon_path.exists() {
            let path_str = icon_path.to_string_lossy();
            let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
            log::info!("Found icon at: {}", path_str);
            return Some(wide_path);
        }
    }
    
    log::warn!("icon.ico not found");
    None
}
