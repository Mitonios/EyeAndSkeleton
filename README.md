# Blink Reminder

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/Windows-10%2B-blue.svg)](https://www.microsoft.com/windows/)

Ứng dụng nhắc nhở chớp mắt và đứng dậy cho người dùng Windows, giúp bảo vệ sức khỏe mắt và xương khớp.

## 📋 Mục lục

- [Tính năng](#-tính-năng)
- [Yêu cầu hệ thống](#-yêu-cầu-hệ-thống)
- [Cài đặt](#-cài-đặt)
- [Cách sử dụng](#-cách-sử-dụng)
- [Cấu trúc dự án](#-cấu-trúc-dự-án)
- [Dependencies](#-dependencies)
- [License](#-license)

## ✨ Tính năng

- **Chạy nền**: Ứng dụng chạy ẩn ở system tray, không hiển thị trên taskbar
- **Nhắc nhở chớp mắt**: Hiển thị animation nhắc nhở theo chu kỳ 1/5/10/30 phút
- **Nhắc nhở đứng dậy**: Hiển thị animation nhắc nhở đứng dậy theo chu kỳ 30/45/60 phút
- **Overlay animation**: Emoji lớn hiển thị ở góc phải trên màn hình, luôn luôn ở trên cùng
- **Cấu hình linh hoạt**: Giao diện config để điều chỉnh các tùy chọn
- **Khởi động tự động**: Tùy chọn chạy cùng Windows
- **Xử lý trùng lặp**: Khi thời gian nhắc đứng dậy trùng chớp mắt, ưu tiên hiển thị nhắc đứng dậy

## 🖥️ Yêu cầu hệ thống

- **Hệ điều hành**: Windows 10/11
- **Bộ nhớ**: RAM 64MB (minimum)
- **Đĩa**: 10MB dung lượng trống
- **Rust**: Rust toolchain stable (phiên bản 1.70+)

## 🚀 Cài đặt

### Clone repository

```bash
git clone https://github.com/your-username/blink-reminder.git
cd blink-reminder
```

### Build ứng dụng

```bash
cargo build --release
```

File executable sẽ được tạo tại `target/release/blink-reminder.exe`

### Chạy ứng dụng

```bash
cargo run --release
```

## 📖 Cách sử dụng

### System Tray Menu

Ứng dụng chạy ẩn ở system tray (biểu tượng gần đồng hồ). Chuột phải vào icon để hiển thị menu:

- **Config...**: Mở cửa sổ cấu hình
- **Exit**: Thoát ứng dụng

### Màn hình cấu hình

Cửa sổ cấu hình (kích thước 400×300) bao gồm:

#### Tùy chọn
- `[ ] Run when Windows starts`: Tự động chạy khi khởi động Windows

#### Thời gian nhắc chớp mắt
- Dropdown chọn khoảng thời gian: 1 phút, 5 phút, 10 phút, 30 phút

#### Thời gian nhắc đứng dậy
- Dropdown chọn khoảng thời gian: 30 phút, 45 phút, 60 phút

#### Các nút điều khiển
- **Test Blink**: Kiểm tra animation nhắc chớp mắt
- **Test Stand Up**: Kiểm tra animation nhắc đứng dậy
- **Save**: Lưu cấu hình và cập nhật timers
- **Exit**: Đóng cửa sổ cấu hình

### Animation nhắc nhở

Khi đến thời gian nhắc nhở:
- **Chớp mắt**: Hiển thị 👁️💧 ở góc phải trên màn hình
- **Đứng dậy**: Hiển thị 🧍⬆️ ở góc phải trên màn hình

Animation sẽ tự động tắt sau 3-5 giây.

## 🏗️ Cấu trúc dự án

```
blink-reminder/
├── src/
│   ├── main.rs              # Entry point, khởi tạo ứng dụng
│   ├── tray.rs              # System tray icon và menu
│   ├── config.rs            # Cấu hình và persistence
│   ├── timer.rs             # Logic timers cho nhắc nhở
│   ├── overlay.rs           # Overlay window và animation
│   ├── config_window.rs     # Giao diện cấu hình
│   └── utils.rs             # Helper functions
├── Cargo.toml               # Dependencies và metadata
└── README.md                # Tài liệu này
```

## 📦 Dependencies

| Mục đích | Crate | Mô tả |
|----------|--------|-------|
| Win32 APIs | `windows` | Giao tiếp với Windows API |
| System tray | `tray-item` | Tạo và quản lý system tray icon |
| GUI config | `egui` + `eframe` | Framework GUI cho cửa sổ cấu hình |
| Async timers | `tokio` | Xử lý timers bất đồng bộ |
| File config | `serde`, `serde_json` | Serialize/deserialize cấu hình JSON |
| App directories | `directories` | Lấy đường dẫn thư mục ứng dụng |

## 📄 License

Dự án này được phân phối dưới giấy phép MIT. Xem file `LICENSE` để biết thêm chi tiết.

## 🔧 Khắc phục sự cố

### Windows Defender cảnh báo

Ứng dụng có thể bị Windows Defender đánh dấu là nghi ngờ vì:
- Chạy nền và truy cập registry
- Tạo overlay windows

**Giải pháp:**
1. Thêm ứng dụng vào danh sách loại trừ của Windows Defender
2. Hoặc compile và ký code với certificate (cho production)

### Quyền truy cập Registry

Nếu gặp lỗi "Quyền truy cập bị từ chối":
- Chạy ứng dụng với quyền Administrator
- Hoặc vô hiệu hóa UAC tạm thời

### Overlay không hiển thị

Nếu overlay emoji không hiện:
- Kiểm tra Windows theme (dark/light mode có thể ảnh hưởng)
- Đảm bảo không có ứng dụng fullscreen che khuất
- Restart ứng dụng

### Config không lưu

Nếu cài đặt không được lưu:
- Kiểm tra quyền ghi vào thư mục `%APPDATA%/blink-reminder/`
- Đóng ứng dụng và mở lại với quyền Administrator

## 🤝 Đóng góp

Mọi đóng góp đều được chào đón! Vui lòng tạo issue hoặc pull request.

## 📞 Liên hệ

Nếu bạn có câu hỏi hoặc gặp vấn đề, vui lòng tạo issue trên GitHub.
