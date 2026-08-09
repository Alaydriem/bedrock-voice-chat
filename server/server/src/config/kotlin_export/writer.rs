use std::collections::BTreeMap;
use std::path::PathBuf;

/// Locates and writes the generated Kotlin config classes in the mods tree.
pub struct KotlinGeneratedFiles;

impl KotlinGeneratedFiles {
    const RELATIVE_PATH: &'static str =
        "mods/java/common/src/main/kotlin/com/alaydriem/bedrockvoicechat/config/generated";

    /// Resolved from the crate manifest rather than the working directory, so
    /// the location does not depend on where cargo was invoked from.
    pub fn output_dir() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest
            .parent()
            .and_then(|server| server.parent())
            .expect("crate manifest sits two levels below the repository root");
        repo_root.join(Self::RELATIVE_PATH)
    }

    pub fn sync(files: &BTreeMap<String, String>) -> Result<Vec<String>, anyhow::Error> {
        let dir = Self::output_dir();
        std::fs::create_dir_all(&dir)?;

        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.ends_with(".kt") && !files.contains_key(&name) {
                std::fs::remove_file(path)?;
            }
        }

        let mut written = Vec::new();
        for (name, body) in files.iter() {
            std::fs::write(dir.join(name), body)?;
            written.push(name.clone());
        }
        Ok(written)
    }
}
