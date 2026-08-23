fn main() {
    // Build-time secrets, the same way the client reads them. `option_env!` resolves
    // against the environment rustc is invoked with, so a value that only exists in a
    // file has to be loaded here and re-emitted below — nothing else in the build sees
    // it.
    //
    // Crate-local first, then the repository root: `from_path` does not overwrite a
    // variable that is already set, so a real environment variable beats `server/`,
    // which beats the root. CI sets the environment and neither file exists there.
    dotenvy::from_path("../.env.local").ok();
    dotenvy::from_path("../../.env.local").ok();
    println!("cargo:rerun-if-changed=../.env.local");
    println!("cargo:rerun-if-changed=../../.env.local");

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
