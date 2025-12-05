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
- **Singleton**: Chỉ cho phép 1 instance chạy - mở lại sẽ hiển thị Config window
- **Nhắc nhở chớp mắt**: Animation nhắc nhở theo chu kỳ 1/5/10/30 phút
- **Nhắc nhở đứng dậy**: Animation nhắc nhở đứng dậy theo chu kỳ 30/45/60 phút
- **Vị trí tùy chọn**: Chọn vị trí hiển thị overlay (góc trên/dưới, trái/phải, giữa màn hình)
- **Overlay animation**: Emoji động hiển thị trong 5 giây, luôn ở trên cùng, bán trong suốt (không che phủ toàn màn hình)
- **Tự động lưu**: Thay đổi cấu hình được lưu ngay lập tức
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

### Build ứng dụng (phát hành exe, không dùng installer)

```bash
cargo build --release
```

File executable nằm tại `target/release/blink-reminder.exe` (kèm `icon.ico` cùng thư mục nếu cần).

### Chạy ứng dụng

```bash
cargo run --release
```

Hoặc chạy trực tiếp file `.exe` (cần copy `icon.ico` cùng thư mục).

## 📖 Cách sử dụng

### System Tray

Ứng dụng chạy ẩn ở system tray (biểu tượng gần đồng hồ):

- **Click trái**: Mở cửa sổ cấu hình
- **Click phải**: Hiển thị menu
  - **Config...**: Mở cửa sổ cấu hình
  - **Exit**: Thoát ứng dụng

### Màn hình cấu hình

Cửa sổ cấu hình tự động mở khi khởi động ứng dụng:

```
┌─────────────────────────────────────┐
│ ☑ Chạy khi Windows khởi động       │
│                                     │
│ Nhắc chớp mắt (phút): [5 ▼] ⏱04:32 │
│ Nhắc đứng dậy (phút): [45▼] ⏱42:15 │
│ Vị trí thông báo:    [Trên - Phải▼]│
│                                     │
│ [Test Chớp mắt] [Test Đứng dậy]    │
│                                     │
│           [ Đóng ]                  │
└─────────────────────────────────────┘
```

#### Tùy chọn

- `☑ Chạy khi Windows khởi động`: Tự động chạy khi khởi động Windows

#### Thời gian nhắc chớp mắt

- Dropdown: 1 phút, 5 phút, 10 phút, 30 phút
- Hiển thị countdown realtime

#### Thời gian nhắc đứng dậy

- Dropdown: 30 phút, 45 phút, 60 phút
- Hiển thị countdown realtime

#### Vị trí thông báo

- Trên - Trái
- Trên - Phải (mặc định)
- Giữa màn hình
- Dưới - Trái
- Dưới - Phải

#### Các nút điều khiển

- **Test Chớp mắt**: Kiểm tra animation nhắc chớp mắt
- **Test Đứng dậy**: Kiểm tra animation nhắc đứng dậy
- **Đóng**: Đóng cửa sổ cấu hình (ứng dụng vẫn chạy nền)

> **Lưu ý**: Cấu hình được **tự động lưu** khi thay đổi, không cần bấm nút Save.

### Animation nhắc nhở

Khi đến thời gian nhắc nhở:

- **Chớp mắt**: 😌 ↔ 🙂 (animation nhắm/mở mắt)
- **Đứng dậy**: 🧍 ↔ 🧎 (animation đứng/quỳ)

Animation hiển thị trong 5 giây rồi tự động tắt.

## 🏗️ Cấu trúc dự án

```
blink-reminder/
├── src/
│   ├── main.rs              # Entry point, event loop chính
│   ├── tray.rs              # System tray icon và menu (Win32 API)
│   ├── config.rs            # Cấu hình và persistence JSON
│   ├── config_window.rs     # Native Win32 config dialog
│   ├── timer.rs             # Dual timer với xử lý trùng lặp
│   ├── overlay.rs           # Transparent overlay với emoji animation
│   ├── registry.rs          # Windows startup registry
│   └── singleton.rs         # Singleton pattern (1 instance)
├── build.rs                 # Embed icon vào executable
├── icon.ico                 # Application icon
├── Cargo.toml               # Dependencies và metadata
└── README.md                # Tài liệu này
```

## 📦 Dependencies

| Mục đích        | Crate                 | Mô tả                           |
| --------------- | --------------------- | ------------------------------- |
| Win32 APIs      | `windows`             | Tray, overlay, registry, dialog |
| Async timers    | `tokio`               | Xử lý timers bất đồng bộ        |
| Config          | `serde`, `serde_json` | Serialize/deserialize JSON      |
| App directories | `directories`         | Đường dẫn %APPDATA%             |
| Error handling  | `anyhow`              | Error handling                  |
| Logging         | `log`, `env_logger`   | Logging                         |
| Build           | `winres`              | Embed icon vào exe              |

## 📄 License

Dự án này được phân phối dưới giấy phép MIT. Xem file `LICENSE` để biết thêm chi tiết.

## 🔧 Khắc phục sự cố

## 🔒 Ký mã số & phát hành sạch

- Build release: `cargo build --release` (không dùng packer/obfuscation/UPX).
- Ký file exe: `signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a target\\release\\blink-reminder.exe`
- Kiểm tra chữ ký: `signtool verify /pa target\\release\\blink-reminder.exe`
- Metadata nhúng: icon + thông tin phiên bản được thiết lập trong `build.rs`.
- Phân phối: chỉ cần phát hành file `blink-reminder.exe` đã ký (không dùng installer).

## 🧪 Kiểm thử & quét AV

1. Quét nội bộ với Windows Defender (Full scan hoặc quét file `.exe`).
2. Gửi mẫu đã ký lên VirusTotal để xem vendor nào gắn cờ.
3. Nếu bị false-positive: gửi mẫu đã ký + mô tả hành vi (tray, overlay, ghi Run key tùy chọn) tới các vendor đó để gỡ cờ.

## 🤔 Khắc phục sự cố

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

### Icon không hiển thị

Nếu tray icon hoặc exe icon không hiện:

- Đảm bảo `icon.ico` nằm cùng thư mục với file `.exe`
- Rebuild với `cargo build --release`

## 🤝 Đóng góp

Mọi đóng góp đều được chào đón! Vui lòng tạo issue hoặc pull request.

## 📞 Liên hệ

Nếu bạn có câu hỏi hoặc gặp vấn đề, vui lòng tạo issue trên GitHub.
