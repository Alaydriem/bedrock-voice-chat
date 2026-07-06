fn main() {
    let src = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("");
    let dst = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built.rs");
    built::write_built_file_with_opts(Some(&src), &dst)
        .expect("Failed to acquire build-time information");

    println!("cargo:rerun-if-env-changed=POSTHOG_KEY");
    if let Ok(key) = std::env::var("POSTHOG_KEY") {
        println!("cargo:rustc-env=POSTHOG_KEY={}", key);
    }
    println!("cargo:rerun-if-env-changed=POSTHOG_HOST");
    if let Ok(host) = std::env::var("POSTHOG_HOST") {
        println!("cargo:rustc-env=POSTHOG_HOST={}", host);
    }
}
