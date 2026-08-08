use crate::i18n::LocalizationService;
use common::structs::i18n::LanguagePack;
use tauri::State;

#[tauri::command]
pub(crate) async fn i18n_locales(
    service: State<'_, LocalizationService>,
) -> Result<Vec<String>, String> {
    Ok(service.available())
}

#[tauri::command]
pub(crate) async fn i18n_load(
    requested: String,
    service: State<'_, LocalizationService>,
) -> Result<Option<LanguagePack>, String> {
    let Some(locale) = service.resolve(&requested) else {
        return Ok(None);
    };
    service.load(&locale).map_err(|e| e.to_string())
}
