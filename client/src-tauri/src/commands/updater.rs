use std::time::Duration;

use tauri_plugin_updater::UpdaterExt;

pub(crate) struct UpdaterHelper;

impl UpdaterHelper {
    // A hung update check used to be bounded only by OS defaults. It holds the in-flight
    // guard in the frontend's UpdateStatus while it waits, which makes the manual
    // "Check again" button do nothing until it resolves.
    const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

    fn is_msix_packaged() -> bool {
        std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.to_str()
                    .map(|s| s.to_lowercase().contains(r"\windowsapps\"))
            })
            .unwrap_or(false)
    }

    fn endpoint() -> &'static str {
        let version = env!("CARGO_PKG_VERSION");
        let is_prerelease =
            version.contains("-beta") || version.contains("-alpha") || version.contains("-rc");

        let endpoint = option_env!("BVC_UPDATER_ENDPOINT").unwrap_or(if is_prerelease {
            "https://www.bedrockvoicechat.com/updater/beta.json"
        } else {
            "https://www.bedrockvoicechat.com/updater/latest.json"
        });

        log::info!(
            "Updater: version={}, channel={}, endpoint={}",
            version,
            if is_prerelease {
                "prerelease"
            } else {
                "stable"
            },
            endpoint
        );

        endpoint
    }

    async fn check(app: &tauri::AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
        let endpoint = Self::endpoint();

        let updater = app
            .updater_builder()
            .timeout(Self::CHECK_TIMEOUT)
            .endpoints(vec![
                endpoint
                    .parse()
                    .map_err(|e: url::ParseError| e.to_string())?,
            ])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;

        updater.check().await.map_err(|e| e.to_string())
    }

    // `version` is the encoded form the updater compares (1.0.523). Manifests published
    // since display_version was added also carry the release's own name (1.0.0-beta.23).
    fn display_version(update: &tauri_plugin_updater::Update) -> String {
        update
            .raw_json
            .get("display_version")
            .and_then(|v| v.as_str())
            .unwrap_or(&update.version)
            .to_string()
    }
}

#[tauri::command]
pub(crate) async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<String>, String> {
    if UpdaterHelper::is_msix_packaged() {
        log::info!("Updater disabled: MSIX-packaged build, updates delivered via Microsoft Store");
        return Ok(None);
    }

    let update = UpdaterHelper::check(&app).await?;

    match update {
        Some(update) => {
            let display = UpdaterHelper::display_version(&update);
            log::info!("Update available: v{} ({})", display, update.version);
            Ok(Some(display))
        }
        None => {
            log::info!("No updates available");
            Ok(None)
        }
    }
}

#[tauri::command]
pub(crate) async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    if UpdaterHelper::is_msix_packaged() {
        return Err(
            "This build is distributed via the Microsoft Store. Updates are delivered automatically by the Store.".to_string(),
        );
    }

    let update = UpdaterHelper::check(&app).await?;

    match update {
        Some(update) => {
            log::info!(
                "Installing update v{} ({})...",
                UpdaterHelper::display_version(&update),
                update.version
            );

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
