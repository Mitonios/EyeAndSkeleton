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
        res.set("LegalCopyright", "Copyright © 2024");

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
