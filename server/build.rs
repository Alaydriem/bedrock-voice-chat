fn main() {
    let src = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("");
    let dst = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("built.rs");
    built::write_built_file_with_opts( Some(&src), &dst)
        .expect("Failed to acquire build-time information");
}