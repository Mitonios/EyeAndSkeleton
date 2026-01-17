use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};

/// Vị trí hiển thị overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OverlayPosition {
    TopLeft,
    #[default]
    TopRight,
    Center,
    BottomLeft,
    BottomRight,
}

impl OverlayPosition {
    /// Lấy tên hiển thị
    pub fn display_name(&self) -> &'static str {
        match self {
            OverlayPosition::TopLeft => "Trên - Trái",
            OverlayPosition::TopRight => "Trên - Phải",
            OverlayPosition::Center => "Giữa màn hình",
            OverlayPosition::BottomLeft => "Dưới - Trái",
            OverlayPosition::BottomRight => "Dưới - Phải",
        }
    }

    /// Lấy tất cả các vị trí
    pub fn all() -> &'static [OverlayPosition] {
        &[
            OverlayPosition::TopLeft,
            OverlayPosition::TopRight,
            OverlayPosition::Center,
            OverlayPosition::BottomLeft,
            OverlayPosition::BottomRight,
        ]
    }

    /// Lấy index trong danh sách
    pub fn index(&self) -> usize {
        Self::all().iter().position(|p| p == self).unwrap_or(1)
    }

    /// Lấy từ index
    pub fn from_index(index: usize) -> Self {
        Self::all().get(index).copied().unwrap_or_default()
    }
}

/// Cấu hình ứng dụng Blink Reminder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Khoảng thời gian nhắc chớp mắt (phút)
    pub blink_interval: u32,

    /// Khoảng thời gian nhắc đứng dậy (phút)
    pub standup_interval: u32,

    /// Vị trí hiển thị overlay
    #[serde(default)]
    pub overlay_position: OverlayPosition,

    /// Ngưỡng idle để reset countdown (phút), 0 = tắt
    #[serde(default = "default_idle_threshold")]
    pub idle_threshold: u32,
}

fn default_idle_threshold() -> u32 {
    2 // Mặc định 2 phút
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            blink_interval: 5,     // 5 phút mặc định
            standup_interval: 45,  // 45 phút mặc định
            overlay_position: OverlayPosition::default(),
            idle_threshold: default_idle_threshold(),
        }
    }
}

impl AppConfig {
    /// Validate cấu hình
    pub fn validate(&self) -> Result<()> {
        // Validate blink interval
        let valid_blink = matches!(self.blink_interval, 1 | 5 | 10 | 30);
        if !valid_blink {
            return Err(anyhow::anyhow!(
                "Blink interval phải là 1, 5, 10 hoặc 30 phút. Giá trị hiện tại: {}",
                self.blink_interval
            ));
        }

        // Validate standup interval
        let valid_standup = matches!(self.standup_interval, 30 | 45 | 60);
        if !valid_standup {
            return Err(anyhow::anyhow!(
                "Stand-up interval phải là 30, 45 hoặc 60 phút. Giá trị hiện tại: {}",
                self.standup_interval
            ));
        }

        // Validate idle threshold (0 = tắt, 1-10 phút)
        if self.idle_threshold > 10 {
            return Err(anyhow::anyhow!(
                "Idle threshold phải từ 0-10 phút. Giá trị hiện tại: {}",
                self.idle_threshold
            ));
        }

        Ok(())
    }

    /// Lấy các giá trị idle threshold hợp lệ
    pub fn valid_idle_thresholds() -> &'static [(u32, &'static str)] {
        &[
            (0, "Tắt"),
            (1, "1 phút"),
            (2, "2 phút"),
            (3, "3 phút"),
            (5, "5 phút"),
            (10, "10 phút"),
        ]
    }
}

/// Lấy đường dẫn đến file config
pub fn get_config_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "blinkreminder", "blink-reminder")
        .context("Không thể lấy thư mục config của ứng dụng")?;

    let config_dir = dirs.config_dir();
    fs::create_dir_all(config_dir)
        .context("Không thể tạo thư mục config")?;

    Ok(config_dir.join("config.json"))
}

/// Tải cấu hình từ file JSON
pub fn load_config() -> Result<AppConfig> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        log::info!("File config không tồn tại, tạo config mặc định");
        let default_config = AppConfig::default();
        save_config(&default_config)?;
        return Ok(default_config);
    }

    let config_str = fs::read_to_string(&config_path)
        .context(format!("Không thể đọc file config tại: {}", config_path.display()))?;

    let config: AppConfig = serde_json::from_str(&config_str)
        .context(format!("File config JSON không hợp lệ tại: {}", config_path.display()))?;

    config.validate()?;
    log::info!("Đã tải config: {:?}", config);

    Ok(config)
}

/// Lưu cấu hình vào file JSON
pub fn save_config(config: &AppConfig) -> Result<()> {
    config.validate()?;

    let config_path = get_config_path()?;
    let config_str = serde_json::to_string_pretty(config)
        .context("Không thể serialize config thành JSON")?;

    fs::write(&config_path, config_str)
        .context("Không thể ghi file config")?;

    log::info!("Đã lưu config: {:?}", config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.blink_interval, 5);
        assert_eq!(config.standup_interval, 45);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid blink interval
        config.blink_interval = 15;
        assert!(config.validate().is_err());

        // Invalid standup interval
        config.blink_interval = 5;
        config.standup_interval = 90;
        assert!(config.validate().is_err());

        // Valid again
        config.standup_interval = 45;
        assert!(config.validate().is_ok());
    }
}
