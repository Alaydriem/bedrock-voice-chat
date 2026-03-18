use tauri_plugin_updater::UpdaterExt;

pub(crate) struct UpdaterHelper;

impl UpdaterHelper {
    fn endpoint() -> &'static str {
        let version = env!("CARGO_PKG_VERSION");
        let is_prerelease = version.contains("-beta")
            || version.contains("-alpha")
            || version.contains("-rc");

        let endpoint = option_env!("BVC_UPDATER_ENDPOINT").unwrap_or(if is_prerelease {
            "https://alaydriem.github.io/bedrock-voice-chat/updater/beta.json"
        } else {
            "https://alaydriem.github.io/bedrock-voice-chat/updater/latest.json"
        });

        log::info!(
            "Updater: version={}, channel={}, endpoint={}",
            version,
            if is_prerelease { "prerelease" } else { "stable" },
            endpoint
        );

        endpoint
    }

    async fn check(
        app: &tauri::AppHandle,
    ) -> Result<Option<tauri_plugin_updater::Update>, String> {
        let endpoint = Self::endpoint();

        let updater = app
            .updater_builder()
            .endpoints(vec![endpoint
                .parse()
                .map_err(|e: url::ParseError| e.to_string())?])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;

        updater.check().await.map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub(crate) async fn check_for_updates(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let update = UpdaterHelper::check(&app).await?;

    match update {
        Some(update) => {
            log::info!("Update available: v{}", update.version);
            Ok(Some(update.version.clone()))
        }
        None => {
            log::info!("No updates available");
            Ok(None)
        }
    }
}

#[tauri::command]
pub(crate) async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let update = UpdaterHelper::check(&app).await?;

    match update {
        Some(update) => {
            log::info!("Installing update v{}...", update.version);

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
        None => Err("No update available".to_string()),
    }
}
