use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub(crate) async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let version = env!("CARGO_PKG_VERSION");
    let is_prerelease = version.contains("-beta")
        || version.contains("-alpha")
        || version.contains("-rc");

    let endpoint = if is_prerelease {
        "https://alaydriem.github.io/bedrock-voice-chat/updater/beta.json"
    } else {
        "https://alaydriem.github.io/bedrock-voice-chat/updater/latest.json"
    };

    log::info!(
        "Updater: version={}, channel={}, endpoint={}",
        version,
        if is_prerelease { "prerelease" } else { "stable" },
        endpoint
    );

    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint.parse().map_err(|e: url::ParseError| e.to_string())?])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;

    let update = updater.check().await.map_err(|e| e.to_string())?;

    match update {
        Some(update) => {
            let new_version = update.version.clone();
            log::info!("Update available: v{new_version}");

            update
                .download_and_install(
                    |chunk_length, content_length| {
                        log::info!("Downloaded {chunk_length} of {content_length:?}");
                    },
                    || {
                        log::info!("Download finished");
                    },
                )
                .await
                .map_err(|e| e.to_string())?;

            log::info!("Update installed, restarting...");
            app.restart();
        }
        None => {
            log::info!("No updates available");
            Ok(None)
        }
    }
}
