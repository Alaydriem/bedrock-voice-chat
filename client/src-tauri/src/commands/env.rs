#[tauri::command]
pub(crate) fn get_env(name: &str) -> String {
    std::env::var(String::from(name)).unwrap_or(String::from(""))
}

#[tauri::command]
pub(crate) fn get_variant() -> String {
    use common::consts::variant::Variant;
    match common::consts::variant::Variant::get() {
        Variant::Dev => "dev".to_string(),
        Variant::Release => "release".to_string(),
    }
}
