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

    tauri_build::build();
}
