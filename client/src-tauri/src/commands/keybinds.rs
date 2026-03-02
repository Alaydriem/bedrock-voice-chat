use common::structs::keybinds::KeybindConfig;

#[cfg(desktop)]
#[tauri::command]
pub(crate) async fn start_keybind_listener(
    config: KeybindConfig,
    km: tauri::State<'_, crate::keybinds::KeybindManager>,
) -> Result<(), String> {
    km.start(config).await;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
pub(crate) async fn start_keybind_listener(_config: KeybindConfig) -> Result<(), String> {
    Ok(())
}
