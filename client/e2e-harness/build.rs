fn main() {
    // Reuse the app crate's icon for the Windows Resource file rather than
    // duplicating the binary asset. The resource is what carries the Windows
    // application manifest the harness needs to launch as a Wry binary.
    tauri_build::try_build(
        tauri_build::Attributes::new().windows_attributes(
            tauri_build::WindowsAttributes::new()
                .window_icon_path("../src-tauri/icons/icon.ico"),
        ),
    )
    .expect("failed to run tauri-build");
}
