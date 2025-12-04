# Kế hoạch xây dựng ứng dụng Blink Reminder

## Kiến trúc tổng quan

Ứng dụng gồm 4 thành phần chính hoạt động song song:

- **Tray App**: Icon system tray + context menu
- **Config Window**: Giao diện cấu hình (egui/eframe)
- **Timer Logic**: 2 timers độc lập (tokio)
- **Overlay Window**: Hiển thị emoji nhắc nhở (Win32 API)

## Giai đoạn 1: Khởi tạo dự án

### 1.1 Tạo Cargo.toml

Dependencies cần thiết:

- `windows` (0.52+) - Win32 APIs cho overlay và registry
- `tray-item` (0.10+) - System tray icon
- `eframe` + `egui` (0.28+) - GUI config window
- `tokio` (1.0+, features: rt-multi-thread, time) - Async timers
- `serde` + `serde_json` - Serialize config
- `directories` - App data paths
- `once_cell` - Global state

### 1.2 Cấu trúc thư mục

```
src/
├── main.rs           # Entry point, event coordination
├── config.rs         # Config struct, load/save JSON
├── tray.rs           # System tray setup
├── timer.rs          # Dual timer logic
├── overlay.rs        # Win32 transparent overlay
├── config_window.rs  # egui config UI
└── registry.rs       # Windows startup registry
```

## Giai đoạn 2: Module Config (config.rs)

- Struct `AppConfig` với serde:
  - `startup: bool` - Chạy cùng Windows
  - `blink_interval: u32` - Phút (1/5/10/30)
  - `standup_interval: u32` - Phút (30/45/60)
- Hàm `load_config()` - Đọc từ `%APPDATA%/blink_reminder/config.json`
- Hàm `save_config()` - Ghi file JSON
- Tạo config mặc định nếu file chưa tồn tại

## Giai đoạn 3: Module Registry (registry.rs)

- Hàm `set_startup(enable: bool)` - Thêm/xóa registry key
- Path: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- Key name: `BlinkReminder`
- Value: Đường dẫn exe hiện tại

## Giai đoạn 4: Module Overlay (overlay.rs)

- Enum `OverlayType { Blink, StandUp }`
- Hàm `show_overlay(overlay_type)`:
  - Tạo Win32 window với flags: `WS_POPUP`, `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT`
  - Vị trí: Góc phải trên màn hình (dùng `GetSystemMetrics`)
  - Render emoji lớn (150-200px):
    - Blink: "👁️" hoặc "💧"
    - StandUp: "🧍" hoặc "🚶"
  - Tự đóng sau 4 giây

## Giai đoạn 5: Module Timer (timer.rs)

- Struct `TimerManager` với 2 interval timers
- Logic xử lý trùng lặp:
  - Theo dõi thời điểm standup tiếp theo
  - Khi blink fire: kiểm tra nếu standup cũng due -> skip blink
- Channel để giao tiếp với main thread
- Hỗ trợ restart timers khi config thay đổi

## Giai đoạn 6: Module Tray (tray.rs)

- Tạo system tray icon (có thể dùng icon mặc định hoặc embed)
- Context menu:
  - "Config..." -> Gửi event mở config window
  - "Exit" -> Gửi event thoát app
- Event handling qua channel

## Giai đoạn 7: Config Window (config_window.rs)

- Sử dụng `eframe` để tạo native window 400x300
- UI với egui:
  - Checkbox "Run when Windows starts"
  - ComboBox "Blink interval": [1, 5, 10, 30] phút
  - ComboBox "Stand-up interval": [30, 45, 60] phút
  - Button "Test Blink" -> Gọi show_overlay(Blink)
  - Button "Test Stand Up" -> Gọi show_overlay(StandUp)
  - Button "Save" -> Lưu config, restart timers
  - Button "Close" -> Đóng window

## Giai đoạn 8: Main Entry Point (main.rs)

- Load config
- Khởi tạo tray icon
- Kiểm tra/cập nhật registry startup
- Spawn timer task (tokio)
- Event loop xử lý:
  - Tray menu events
  - Timer events -> Show overlay
  - Config window events
- Cleanup khi exit

## Thứ tự triển khai đề xuất

1. **Cargo.toml** + **main.rs** skeleton
2. **config.rs** - Có thể test độc lập
3. **registry.rs** - Test đọc/ghi registry
4. **overlay.rs** - Test hiển thị emoji
5. **timer.rs** - Test interval logic
6. **tray.rs** - Test menu popup
7. **config_window.rs** - Tích hợp egui
8. **Kết nối tất cả** trong main.rs

## Rủi ro và giải pháp

| Rủi ro | Giải pháp |

|--------|-----------|

| Multi-monitor | Dùng `GetSystemMetrics(SM_CXSCREEN)` cho primary |

| Overlay bị che | Dùng `HWND_TOPMOST` và `SetWindowPos` |

| Timer drift | Dùng `tokio::time::interval` với `MissedTickBehavior::Skip` |

| Config race | Dùng `Arc<RwLock<Config>>` cho shared state |
