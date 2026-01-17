//! Build script để embed icon và generate build info

use std::process::Command;

fn main() {
    // Generate build info (timestamp + git hash)
    let timestamp = chrono_lite_timestamp();
    let git_hash = get_git_hash();
    let build_info = format!("{} ({})", timestamp, git_hash);

    println!("cargo:rustc-env=BUILD_INFO={}", build_info);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=BUILD_GIT_HASH={}", git_hash);

    // Rebuild nếu git HEAD thay đổi
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    // Chỉ build resource trên Windows
    #[cfg(target_os = "windows")]
    {
        // Embed icon vào executable
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set_manifest_file("app.manifest");
        res.set("ProductName", "Blink Reminder");
        res.set("FileDescription", "Ứng dụng nhắc nhở chớp mắt và đứng dậy");
        res.set("CompanyName", "Mitonios with AI");
        res.set("OriginalFilename", "blink-reminder.exe");
        res.set("InternalName", "blink-reminder");
        res.set("FileVersion", "1.1.0.0");
        res.set("ProductVersion", "1.1.0");
        res.set("LegalCopyright", "Copyright © 2025 Mitonios with AI");

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}

/// Lấy git commit hash ngắn (7 ký tự)
fn get_git_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Generate timestamp theo format YYYYMMDD.HHMM (không cần chrono crate)
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Tính toán thủ công (UTC+7 cho Vietnam)
    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_min = 60u64;

    // Offset UTC+7
    let local_secs = now + 7 * secs_per_hour;

    // Tính ngày từ epoch (1970-01-01)
    let days = local_secs / secs_per_day;
    let remaining = local_secs % secs_per_day;
    let hours = remaining / secs_per_hour;
    let mins = (remaining % secs_per_hour) / secs_per_min;

    // Thuật toán tính năm/tháng/ngày từ số ngày
    let (year, month, day) = days_to_ymd(days);

    format!("{:04}{:02}{:02}.{:02}{:02}", year, month, day, hours, mins)
}

/// Chuyển đổi số ngày từ epoch thành (year, month, day)
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Số ngày từ 1970-01-01
    let mut remaining_days = days as i64;

    // Bắt đầu từ năm 1970
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Tìm tháng
    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1i64;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1; // Ngày bắt đầu từ 1

    (year as u64, month as u64, day as u64)
}

/// Kiểm tra năm nhuận
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
