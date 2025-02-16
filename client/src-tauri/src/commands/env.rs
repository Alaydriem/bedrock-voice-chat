#[tauri::command]
pub(crate) fn get_env(name: &str) -> String {
    std::env::var(String::from(name)).unwrap_or(String::from(""))
}

#[tauri::command]
pub(crate) fn get_variant() -> String {
    if cfg!(dev) {
        return String::from("dev");
    } else {
        return String::from("release");
    }
}
