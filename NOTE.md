# Development Notes

## Đã hoàn thành: Idle Detection & Reset Countdown

### Tính năng
Khi user idle (không có input keyboard/mouse) vượt ngưỡng cài đặt:
- Tạm dừng không hiển thị thông báo
- Khi user trở lại hoạt động → reset countdown về từ đầu

### Cách hoạt động
1. **Idle check mỗi 10 giây** sử dụng `GetLastInputInfo()` Win32 API
2. **Ngưỡng idle** có thể cấu hình: Tắt, 1, 2, 3, 5, 10 phút (mặc định 2 phút)
3. **Khi user idle**:
   - Không hiển thị thông báo blink/standup
   - Đánh dấu trạng thái `was_idle = true`
4. **Khi user trở lại**:
   - Reset cả 2 timer về countdown đầy đủ
   - Log: "User trở lại từ idle, reset countdown"

### Files thay đổi
- `Cargo.toml`: Thêm `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_SystemInformation`
- `config.rs`: Thêm `idle_threshold` field, validation, options
- `timer.rs`: Thêm `get_idle_duration()`, idle check logic, `reset_all_timers()`
- `config_window.rs`: Thêm UI dropdown cho "Reset khi idle"

---

## Đã hoàn thành: Fix viền sáng animation (Direct2D)

### Vấn đề
Animation emoji hiển thị viền sáng (halo) khi nền bên dưới là màu tối.

### Nguyên nhân
- Code cũ sử dụng GDI với Color Key transparency (`LWA_COLORKEY`)
- Chỉ loại bỏ pixel **chính xác** màu trắng (0xFFFFFF)
- Anti-aliased pixels xung quanh emoji có màu xám nhạt → không bị loại bỏ → tạo viền sáng

### Giải pháp đã triển khai
Thay thế GDI bằng Direct2D/DirectWrite với per-pixel alpha blending:

1. **Per-pixel Alpha** thay vì Color Key:
   - `UpdateLayeredWindow` với `ULW_ALPHA`
   - 32-bit ARGB DIB section để lưu pixel với alpha channel

2. **Direct2D DCRenderTarget**:
   - Pixel format: `DXGI_FORMAT_B8G8R8A8_UNORM` + `D2D1_ALPHA_MODE_PREMULTIPLIED`
   - Clear background với alpha = 0.0 (hoàn toàn trong suốt)

3. **DirectWrite cho text**:
   - `D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT` để render color emoji

### Kết quả
- Emoji hiển thị mượt mà trên mọi nền
- Không còn viền sáng trên nền tối
- Anti-aliased pixels blend chính xác với nền
