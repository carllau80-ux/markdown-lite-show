fn main() {
    // Build scripts run on the host, so use Cargo's target description rather
    // than #[cfg(windows)] (which would be false when cross-compiling).
    let target_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    // On native Windows the MSVC resource compiler is discovered by winresource.
    // When cross-compiling, only try if an explicit windres was provided.
    let resource_tool_available = cfg!(windows) || std::env::var_os("WINDRES").is_some();
    if target_windows && resource_tool_available {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource.set("ProductName", "MarkLite");
        resource.set("FileDescription", "MarkLite - lightweight Markdown editor");
        resource.set("CompanyName", "MarkLite");
        resource.set("LegalCopyright", "MIT License");
        resource.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        resource.set("FileVersion", env!("CARGO_PKG_VERSION"));
        if let Err(error) = resource.compile() {
            println!("cargo:warning=Windows resource compilation failed: {error}");
        }
    }
}
