# **1. Tổng quan kiến trúc**

Ứng dụng gồm 4 phần chính:

1. **Tray App (background + icon + context menu)**
2. **Config Window (UI để chỉnh cấu hình)**
3. **Scheduler/Timers (logic nhắc nhở)**
4. **Overlay Window luôn-on-top (hiển thị animation nhắc nhở)**

Bạn không cần một framework GUI nặng. Phù hợp nhất cho Rust + Windows hiện tại:

- **Tray icon**: crate `tray-item` hoặc `win-tray-icon`

- **Config UI**:

  - Dễ nhất: `egui` + `eframe` (tự render UI vào 1 window native)
  - Hoặc `native-windows-gui (nwg)` nếu bạn muốn UI Win32 truyền thống

- **Overlay animation**:

  - Tạo **borderless transparent top-most window** bằng Win32 API qua crate `windows`.
  - Vẽ emoji/ảnh động bằng `egui` hoặc GDI+.

- **Startup with Windows**:

  - Tạo registry key tại
    `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`

- **Lưu cấu hình**:

  - File JSON trong `%APPDATA%/blink_reminder/config.json`

---

# **2. Chức năng chi tiết**

## **2.1. Ứng dụng chạy nền ở taskbar (system tray)**

- Khi chạy ứng dụng → không xuất hiện ở taskbar, chỉ là icon ở system tray.

- Chuột phải vào icon sẽ hiện menu:

  - **Config…**
  - **Exit**

- Khi bấm **Exit** → đóng timers, lưu config, thoát hẳn.

## **2.2. Màn hình Config**

Một cửa sổ nhỏ (400×300) gồm:

### **A. Checkbox**

- `[ ] Run when Windows starts`

### **B. Dropdown (chớp mắt)**

- "Blink reminder interval":

  - 1 phút
  - 5 phút
  - 10 phút
  - 30 phút

### **C. Dropdown (đứng dậy)**

- "Stand-up reminder interval":

  - 30 phút
  - 45 phút
  - 60 phút

### **D. Buttons**

- **Test Blink**
- **Test Stand Up**
- **Save**
- **Exit (Close config window)**

Nút "Save" lưu vào file config và cập nhật timer đang chạy.

---

# **3. Timer Logic (điểm quan trọng)**

Bạn cần chạy **2 timers riêng biệt**:

- `blink_timer`
- `standup_timer`

Khi đếm đến 0 → kích hoạt overlay tương ứng.

### **Rule tránh trùng lặp**

Khi **blink** và **stand-up** đến cùng lúc:

- Chỉ hiện **stand-up**.
- Blink timer sẽ reset sau khi stand-up notification kết thúc.

### Timer implementation

Bạn có 3 lựa chọn:

1. **tokio + interval**
2. **std::thread + sleep**
3. **windows timer queue** (native Win32)

Đơn giản nhất: **tokio runtime**.

---

# **4. Overlay Animation (nhắc nhở)**

### **Yêu cầu**

- Hiển thị ở góc phải phía trên màn hình.
- Luôn top-most.
- Trong suốt nền (transparent).
- Chỉ có emoji/animation.
- Tự tắt sau vài giây.

### **Hiển thị gì?**

Ví dụ:

- Blink: 👁️💧 (hoặc 2–3 frame blink)
- Stand-up: 🧍⬆️ hoặc 🕺 (icon to)

### **Cách làm**

- Tạo một cửa sổ Win32:

  - `WS_POPUP`
  - `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT`

- Render emoji lớn bằng:

  - `DirectWrite` (qua crate `windows`)
  - Hoặc đơn giản hơn: dùng `egui` vẽ Unicode emoji size 150–300px.

### **Đời sống thực**

Emoji native Windows hiển thị rất đẹp, không cần PNG.

### **Tự tắt**

- Overlay sống 3–5 giây (config được).
- Sau đó tự hủy window.

---

# **5. Khởi động cùng Windows**

Khi bật tùy chọn:

```reg
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
name = "BlinkApp"
value = "C:\path\to\app.exe"
```

Tắt tùy chọn → xóa key.

---

# **6. File cấu hình**

Ví dụ cấu hình JSON:

```json
{
  "startup": true,
  "blink_interval": 5,
  "standup_interval": 45
}
```

Lưu ở:

```
%APPDATA%/blink_reminder/config.json
```

---

# **7. Flow hoạt động**

1. App khởi động → load config.
2. Tạo system tray icon + menu.
3. Nếu config.startup = true → kiểm tra registry.
4. Khởi tạo timers:

   - blink_timer = interval(config.blink_interval)
   - standup_timer = interval(config.standup_interval)

5. Khi timer fire:

   - Kiểm tra trùng nhau.
   - Hiển thị overlay animation tương ứng.

6. User mở Config → điều chỉnh → Save → cập nhật timer.
7. Exit từ tray menu → đóng mọi thread → thoát.

---

# **8. Gợi ý thư viện Rust**

| Mục đích                   | Crate                                          |
| -------------------------- | ---------------------------------------------- |
| Win32 APIs                 | `windows`                                      |
| System tray                | `tray-item` hoặc `win-tray-icon`               |
| GUI config                 | `egui` + `eframe`                              |
| Async timers               | `tokio` hoặc `async-std`                       |
| File config                | `serde`, `serde_json`, `directories`           |
| Transparent overlay window | dùng `windows::Win32::UI::WindowsAndMessaging` |

---

# **9. Dàn mã pseudo code**

### **main.rs**

```rust
fn main() {
    let config = load_config();

    init_tray_menu();

    if config.startup {
        ensure_registry_startup();
    }

    start_timers(config);

    event_loop();
}
```

### Timer logic

```rust
async fn start_timers(config: Config) {
    let mut blink = tokio::time::interval(Duration::from_minutes(config.blink));
    let mut stand = tokio::time::interval(Duration::from_minutes(config.stand));

    loop {
        tokio::select! {
            _ = blink.tick() => {
                if stand_due_now() { continue; }
                show_overlay(OverlayType::Blink);
            }
            _ = stand.tick() => {
                show_overlay(OverlayType::Stand);
            }
        }
    }
}
```

### Overlay

```rust
fn show_overlay(ty: OverlayType) {
    create_layered_window();
    render_emoji(ty);
    sleep(4 secs);
    destroy_window();
}
```

---

# **10. Những rủi ro cần xử lý**

- **Nhiều màn hình**
  Bạn phải lấy tọa độ màn hình chính (`GetSystemMetrics(SM_CXSCREEN)`).

- **Windows Defender cảnh báo**
  Vì app chạy nền → nên ký code nếu bạn phát hành.

- **Góc phải bị che bởi HUD app khác**
  Giải pháp: cho phép config vị trí overlay.

- **Trùng timer blink/stand-up**
  Đã xử lý bằng rule ưu tiên stand-up.

---
