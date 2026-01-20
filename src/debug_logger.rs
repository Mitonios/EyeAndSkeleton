use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

/// Ghi log vào file debug_log.txt
pub fn log_event(message: &str) {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let log_line = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug_log.txt")
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}
