use crate::keybinds::KeybindManager;
use common::structs::keybinds::KeybindConfig;
use tauri::State;

#[tauri::command]
pub(crate) async fn start_keybind_listener(
    config: KeybindConfig,
    km: State<'_, KeybindManager>,
) -> Result<(), String> {
    km.start(config).await;
    Ok(())
}
