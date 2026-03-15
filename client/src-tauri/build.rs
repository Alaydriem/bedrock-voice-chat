fn main() {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=BUILD_COMMIT={}", sha);
        }
        _ => {
            println!("cargo:rustc-env=BUILD_COMMIT=unknown");
        }
    }

    println!("cargo:rerun-if-changed=../../.git/HEAD");

    println!("cargo:rerun-if-env-changed=SENTRY_DSN");
    if let Ok(dsn) = std::env::var("SENTRY_DSN") {
        println!("cargo:rustc-env=SENTRY_DSN={}", dsn);
    }

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    println!("cargo:rerun-if-env-changed=IOS_BUILD_NUMBER");
    println!("cargo:rerun-if-env-changed=ANDROID_VERSION_CODE");
    println!("cargo:rerun-if-env-changed=MACOS_BUILD_NUMBER");

    let release = match target_os.as_str() {
        "ios" => std::env::var("IOS_BUILD_NUMBER").unwrap_or_else(|_| version.clone()),
        "android" => std::env::var("ANDROID_VERSION_CODE").unwrap_or_else(|_| version.clone()),
        "macos" => std::env::var("MACOS_BUILD_NUMBER").unwrap_or_else(|_| version.clone()),
        _ => version,
    };
    println!("cargo:rustc-env=SENTRY_RELEASE={release}");

    tauri_build::build();
}
