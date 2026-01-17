# Blink Reminder

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![Windows](https://img.shields.io/badge/Windows-10%2B-blue.svg)](https://www.microsoft.com/windows/)
[![Version](https://img.shields.io/badge/version-1.1.0-blue.svg)](https://github.com/your-username/blink-reminder)

Ứng dụng nhắc nhở chớp mắt và đứng dậy cho người dùng Windows, giúp bảo vệ sức khỏe mắt và xương khớp.

## Mục lục

- [Tính năng](#tính-năng)
- [Yêu cầu hệ thống](#yêu-cầu-hệ-thống)
- [Cài đặt](#cài-đặt)
- [Cách sử dụng](#cách-sử-dụng)
- [Cấu trúc dự án](#cấu-trúc-dự-án)
- [Dependencies](#dependencies)
- [License](#license)

## Tính năng

- **Chạy nền**: Ứng dụng chạy ẩn ở system tray, không hiển thị trên taskbar
- **Singleton**: Chỉ cho phép 1 instance chạy - mở lại sẽ hiển thị Config window
- **Nhắc nhở chớp mắt**: Animation nhắc nhở theo chu kỳ 1/5/10/30 phút
- **Nhắc nhở đứng dậy**: Animation nhắc nhở đứng dậy theo chu kỳ 30/45/60 phút
- **Vị trí tùy chọn**: Chọn vị trí hiển thị overlay (góc trên/dưới, trái/phải, giữa màn hình)
- **Overlay animation**: Emoji động hiển thị trong 5 giây, luôn ở trên cùng, trong suốt hoàn hảo (Direct2D)
- **Chống spam**: Chỉ hiển thị 1 overlay tại một thời điểm, tránh chồng chéo
- **Tự động lưu**: Thay đổi cấu hình được lưu ngay lập tức
- **DPI Aware**: Hỗ trợ hiển thị sắc nét trên màn hình High-DPI
- **Xử lý trùng lặp**: Khi thời gian nhắc đứng dậy trùng chớp mắt, ưu tiên hiển thị nhắc đứng dậy và reset blink timer

## Yêu cầu hệ thống

- **Hệ điều hành**: Windows 10/11
- **Bộ nhớ**: RAM 64MB (minimum)
- **Đĩa**: 10MB dung lượng trống
- **Rust**: Rust toolchain stable (phiên bản 1.70+)

## Cài đặt

### Clone repository

```bash
git clone https://github.com/your-username/blink-reminder.git
cd blink-reminder
```

### Build ứng dụng

```bash
cargo build --release
```

File executable nằm tại `target/release/blink-reminder.exe` (kèm `icon.ico` cùng thư mục nếu cần).

### Chạy ứng dụng

```bash
cargo run --release
```

Hoặc chạy trực tiếp file `.exe` (cần copy `icon.ico` cùng thư mục).

## Cách sử dụng

### System Tray

Ứng dụng chạy ẩn ở system tray (biểu tượng gần đồng hồ):

- **Click trái**: Mở cửa sổ cấu hình
- **Click phải**: Hiển thị menu
  - **Cấu hình...**: Mở cửa sổ cấu hình
  - **Thoát**: Thoát ứng dụng

### Màn hình cấu hình

Cửa sổ cấu hình sử dụng egui framework với giao diện hiện đại:

```
┌─────────────────────────────────────────┐
│ Blink Reminder - Cấu hình               │
├─────────────────────────────────────────┤
│                                         │
│ Nhắc chớp mắt (phút): [5 ▼] 04:32 [Test]│
│                                         │
│ Nhắc đứng dậy (phút): [45▼] 42:15 [Test]│
│                                         │
│ Vị trí thông báo: [Giữa màn hình ▼]     │
│                                         │
│─────────────────────────────────────────│
│ Phiên bản: 1.1.0                        │
│ Tác giả: Mitonios            [Thoát]    │
└─────────────────────────────────────────┘
```

#### Thời gian nhắc chớp mắt

- Dropdown: 1 phút, 5 phút, 10 phút, 30 phút
- Hiển thị countdown realtime (mm:ss)
- Nút **Test**: Kiểm tra animation nhắc chớp mắt

#### Thời gian nhắc đứng dậy

- Dropdown: 30 phút, 45 phút, 60 phút
- Hiển thị countdown realtime (mm:ss)
- Nút **Test**: Kiểm tra animation nhắc đứng dậy

#### Vị trí thông báo

- Trên - Trái
- Trên - Phải
- Giữa màn hình (mặc định)
- Dưới - Trái
- Dưới - Phải

#### Các nút điều khiển

- **Thoát**: Đóng hoàn toàn ứng dụng
- **Nút X (đóng cửa sổ)**: Thu nhỏ xuống tray (ứng dụng vẫn chạy nền)

> **Lưu ý**: Cấu hình được **tự động lưu** khi thay đổi, không cần bấm nút Save.

### Animation nhắc nhở

Khi đến thời gian nhắc nhở, overlay 400x400 pixel hiển thị:

- **Chớp mắt**: 😌 ↔ 🙂 (animation nhắm/mở mắt)
- **Đứng dậy**: 🧍 ↔ 🧎 (animation đứng/quỳ)

Animation hiển thị trong 5 giây rồi tự động tắt.

## Kiến trúc ứng dụng

```
┌─────────────────────────────────────────────────────────────┐
│                      Main Thread                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              egui/eframe Config Window               │    │
│  │         (poll messages mỗi 100ms)                   │    │
│  └─────────────────────────────────────────────────────┘    │
│                          ▲                                   │
│                          │ std::sync::mpsc                   │
│                          │                                   │
│  ┌───────────────────────┴───────────────────────────┐      │
│  │              Tray Event Handler Thread             │      │
│  └───────────────────────┬───────────────────────────┘      │
│                          │                                   │
└──────────────────────────┼──────────────────────────────────┘
                           │ std::sync::mpsc
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Background Threads                        │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │   Tray Thread   │  │  Tokio Runtime  │                   │
│  │  (Win32 msg     │  │  ┌───────────┐  │                   │
│  │   loop)         │  │  │  Timer    │  │                   │
│  └─────────────────┘  │  │  Manager  │  │                   │
│                       │  └───────────┘  │                   │
│                       └─────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

## Cấu trúc dự án

```
blink-reminder/
├── src/
│   ├── main.rs              # Entry point, khởi tạo threads và channels
│   ├── tray.rs              # System tray icon và menu (Win32 API)
│   ├── config.rs            # Cấu hình và persistence JSON
│   ├── config_window.rs     # Config window sử dụng egui/eframe
│   ├── timer.rs             # Dual timer với tokio async
│   ├── overlay.rs           # Transparent overlay với Direct2D/DirectWrite
│   └── singleton.rs         # Singleton pattern và IPC
├── build.rs                 # Embed icon và manifest vào executable
├── app.manifest             # DPI awareness manifest
├── icon.ico                 # Application icon
├── Cargo.toml               # Dependencies và metadata
└── README.md                # Tài liệu này
```

## Dependencies

| Mục đích        | Crate                 | Mô tả                              |
| --------------- | --------------------- | ---------------------------------- |
| GUI Framework   | `eframe`, `egui`      | Cross-platform immediate mode GUI  |
| Win32 APIs      | `windows`             | Tray, overlay, singleton, Direct2D |
| Async Runtime   | `tokio`               | Xử lý timers bất đồng bộ           |
| Config          | `serde`, `serde_json` | Serialize/deserialize JSON         |
| App directories | `directories`         | Đường dẫn %APPDATA%                |
| Error handling  | `anyhow`              | Error handling                     |
| Logging         | `log`, `env_logger`   | Logging (RUST_LOG=info để debug)   |
| Build           | `winres`              | Embed icon và manifest vào exe     |

## License

Dự án này được phân phối dưới giấy phép MIT. Xem file `LICENSE` để biết thêm chi tiết.

## Ký mã số & phát hành

- Build release: `cargo build --release`
- Ký file exe: `signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a target\release\blink-reminder.exe`
- Kiểm tra chữ ký: `signtool verify /pa target\release\blink-reminder.exe`
- Phân phối: chỉ cần phát hành file `blink-reminder.exe` đã ký

## Khắc phục sự cố

### Windows Defender cảnh báo

Ứng dụng có thể bị Windows Defender đánh dấu là nghi ngờ vì:

- Chạy nền và tạo system tray icon
- Tạo overlay windows

**Giải pháp:**

1. Thêm ứng dụng vào danh sách loại trừ của Windows Defender
2. Hoặc ký code với certificate (cho production)

### Overlay không hiển thị

Nếu overlay emoji không hiện:

- Đảm bảo không có ứng dụng fullscreen che khuất
- Restart ứng dụng
- Kiểm tra GPU driver (Direct2D yêu cầu driver đồ họa hoạt động)

### Config không lưu

Nếu cài đặt không được lưu:

- Kiểm tra quyền ghi vào thư mục `%APPDATA%/blink-reminder/`
- Chạy với logging: `RUST_LOG=info blink-reminder.exe`

### Icon không hiển thị

Nếu tray icon không hiện:

- Đảm bảo `icon.ico` nằm cùng thư mục với file `.exe`
- Rebuild với `cargo build --release`

### Font tiếng Việt không hiển thị

Ứng dụng tự động load font Segoe UI từ hệ thống Windows. Nếu vẫn lỗi:

- Đảm bảo font `C:\Windows\Fonts\segoeui.ttf` tồn tại
- Thử cài đặt lại font Segoe UI

## Đóng góp

Mọi đóng góp đều được chào đón! Vui lòng tạo issue hoặc pull request.

## Liên hệ

Nếu bạn có câu hỏi hoặc gặp vấn đề, vui lòng tạo issue trên GitHub.
