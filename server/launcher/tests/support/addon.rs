use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Builds `.mcaddon` and `.mcpack` archives shaped like the released ones.
pub struct Addon;

impl Addon {
    pub const BEHAVIOR_UUID: &'static str = "6fb24263-357a-407c-aebe-681f50b2de50";
    pub const RESOURCE_UUID: &'static str = "20a36865-747e-4c38-8077-ea9228c2bd5d";
    pub const VERSION: [u64; 3] = [1, 0, 521];

    /// A pack whose manifest header names `uuid` and `version`.
    pub fn mcpack(uuid: &str, version: [u64; 3]) -> Vec<u8> {
        let manifest = format!(
            r#"{{"format_version":2,"header":{{"name":"pack.name","uuid":"{uuid}","version":[{},{},{}]}}}}"#,
            version[0], version[1], version[2]
        );
        Self::zip(&[("manifest.json", manifest.as_bytes())])
    }

    /// The released add-on: both packs at `VERSION`.
    pub fn released(path: &Path) {
        let behavior = Self::mcpack(Self::BEHAVIOR_UUID, Self::VERSION);
        let resource = Self::mcpack(Self::RESOURCE_UUID, Self::VERSION);
        Self::build(
            path,
            &[
                ("bedrock-voice-chat-bp.mcpack", &behavior),
                ("bedrock-voice-chat-rp.mcpack", &resource),
            ],
        );
    }

    pub fn build(path: &Path, entries: &[(&str, &[u8])]) {
        File::create(path)
            .unwrap()
            .write_all(&Self::zip(entries))
            .unwrap();
    }

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
}
