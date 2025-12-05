use anyhow::{Context, Result};
use std::path::PathBuf;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;

/// Tên key trong registry cho startup
const REG_KEY_NAME: &str = "BlinkReminder";

/// Thiết lập hoặc xóa registry key cho Windows startup
pub fn set_startup(enable: bool) -> Result<()> {
    let exe_path = get_current_exe_path()?;

    unsafe {
        // Mở registry key HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run
        let mut hkey = HKEY::default();
        let reg_path = HSTRING::from("Software\\Microsoft\\Windows\\CurrentVersion\\Run");

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR::from_raw(reg_path.as_ptr()),
            Some(0),
            KEY_SET_VALUE | KEY_QUERY_VALUE,
            &mut hkey,
        );
        if result != ERROR_SUCCESS {
            return Err(anyhow::anyhow!("Không thể mở registry key Run: {:?}", result));
        }

        if enable {
            // Thêm registry value
            let exe_path_str = exe_path.to_string_lossy();
            let key_name = HSTRING::from(REG_KEY_NAME);

            // Convert path to wide bytes (null-terminated)
            let value_bytes: Vec<u16> = exe_path_str.encode_utf16().chain(std::iter::once(0)).collect();

            let result = RegSetValueExW(
                hkey,
                PCWSTR::from_raw(key_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    value_bytes.as_ptr() as *const u8,
                    value_bytes.len() * 2,
                )),
            );

            if result != ERROR_SUCCESS {
                let _ = RegCloseKey(hkey);
                return Err(anyhow::anyhow!(
                    "Không thể thiết lập registry value: {:?}",
                    result
                ));
            }

            log::info!(
                "Đã thêm startup registry: {} -> {}",
                REG_KEY_NAME,
                exe_path_str
            );
        } else {
            // Xóa registry value
            let key_name = HSTRING::from(REG_KEY_NAME);

            match RegDeleteValueW(hkey, PCWSTR::from_raw(key_name.as_ptr())) {
                ERROR_SUCCESS => log::info!("Đã xóa startup registry: {}", REG_KEY_NAME),
                ERROR_FILE_NOT_FOUND => {}
                err => {
                    let _ = RegCloseKey(hkey);
                    return Err(anyhow::anyhow!("Không thể xóa registry value: {:?}", err));
                }
            }
        }

        let _ = RegCloseKey(hkey);
    }

    Ok(())
}

/// Lấy đường dẫn đến executable hiện tại
fn get_current_exe_path() -> Result<PathBuf> {
    std::env::current_exe().context("Không thể lấy đường dẫn executable hiện tại")
}
