//! Build script để embed icon vào executable

fn main() {
    // Chỉ build resource trên Windows
    #[cfg(target_os = "windows")]
    {
        // Embed icon vào executable
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Blink Reminder");
        res.set("FileDescription", "Ứng dụng nhắc nhở chớp mắt và đứng dậy");
        res.set("CompanyName", "Mitonios with AI");
        res.set("OriginalFilename", "blink-reminder.exe");
        res.set("InternalName", "blink-reminder");
        res.set("FileVersion", "1.0.0.0");
        res.set("ProductVersion", "1.0.0");
        res.set("LegalCopyright", "Copyright © 2025 Mitonios with AI");

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
