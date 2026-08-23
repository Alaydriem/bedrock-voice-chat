use bvc_server_lib::config::{KotlinExporter, KotlinGeneratedFiles};

const REGENERATE: &str = "UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export";

// Line endings differ between a checked-in file and a freshly built string on
// Windows; comparing normalized text keeps that from reading as drift.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[test]
fn generated_kotlin_matches_the_checked_in_files() {
    let files = KotlinExporter::new().export().expect("export succeeds");
    let dir = KotlinGeneratedFiles::output_dir();

    if std::env::var("UPDATE_KOTLIN_CONFIG").is_ok() {
        let written = KotlinGeneratedFiles::sync(&files).expect("writing generated files");
        println!("wrote {} generated Kotlin files", written.len());
        return;
    }

    for (name, expected) in files.iter() {
        let path = dir.join(name);
        let actual = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} is missing. Regenerate with: {REGENERATE}", path.display()));
        assert_eq!(
            normalize(&actual),
            normalize(expected),
            "{} is out of date. Regenerate with: {REGENERATE}",
            path.display()
        );
    }
}

// A class deleted from the Rust config leaves an orphan Kotlin file that still
// compiles, so the absence check is as important as the content check.
#[test]
fn no_orphan_generated_files_remain() {
    if std::env::var("UPDATE_KOTLIN_CONFIG").is_ok() {
        return;
    }

    let files = KotlinExporter::new().export().expect("export succeeds");
    let dir = KotlinGeneratedFiles::output_dir();
    let entries = std::fs::read_dir(&dir).expect("generated directory exists");

    for entry in entries {
        let name = entry.expect("readable entry").file_name();
        let name = name.to_string_lossy().to_string();
        if !name.ends_with(".kt") {
            continue;
        }
        assert!(
            files.contains_key(&name),
            "{name} is no longer generated. Regenerate with: {REGENERATE}"
        );
    }
}
