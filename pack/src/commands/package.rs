use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Seek},
    path::Path,
    sync::Arc,
};
use walkdir::{DirEntry, WalkDir};

use super::Command;
use std::io::Write;
use zip::{result::ZipError, write::FileOptions};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(rename = "format_version")]
    pub format_version: i64,
    pub header: Header,
    pub modules: Vec<Module>,
    pub dependencies: Option<Vec<Dependency>>,
    pub metadata: Metadata,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub name: String,
    pub description: String,
    pub uuid: String,
    pub version: Vec<i64>,
    #[serde(rename = "min_engine_version")]
    pub min_engine_version: Vec<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    #[serde(rename = "type")]
    pub type_field: String,
    pub uuid: String,
    pub version: Vec<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub version: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(rename ="module_name", skip_serializing_if = "Option::is_none")]
    pub module_name: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub authors: Vec<String>,
}

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run<'a>(&'a self, _cfg: &Arc<Command>) {
        let bp_path = Path::new("pack/bp");
        let rp_path = Path::new("pack/rp");
        let src = Path::new("src");
        let paths = vec![bp_path, rp_path];

        let package_name = env!("CARGO_PKG_NAME")
            .to_string();

        let description = env!("CARGO_PKG_DESCRIPTION")
            .to_string();

        let major = env!("CARGO_PKG_VERSION_MAJOR")
            .to_string()
            .parse::<i64>()
            .unwrap();

        let minor = env!("CARGO_PKG_VERSION_MINOR")
            .to_string()
            .parse::<i64>()
            .unwrap();

        let patch = env!("CARGO_PKG_VERSION_PATCH")
            .to_string()
            .parse::<i64>()
            .unwrap();

        let mut txt = fs::read_to_string(src.join("js/version.js")).unwrap();
        txt = txt.replace("__VERSION__", format!("{}.{}.{}", major, minor, patch).as_ref());
        fs::write(bp_path.join("scripts/version.js"), txt).unwrap();

        for path in paths {
            // Modify the version from the cargo.toml manifest
            let mut manifest = fs::File::open(path.join("manifest.json")).unwrap();
            let mut buf: String = String::new();
            match manifest.read_to_string(&mut buf) {
                Ok(_) => {}
                Err(err) => panic!("{}", err.to_string()),
            };

            let mut json: Manifest = match serde_json::from_str(&buf) {
                Ok(json) => match json {
                    Some(json) => json,
                    None => panic!("No data in buffer"),
                },
                Err(err) => panic!("Could not read JSON file: {}", err.to_string()),
            };

            json.header.version = vec![major, minor, patch];

            for (i, _) in json.modules.clone().iter().enumerate() {
                json.modules[i].version = vec![major, minor, patch];
            }

            json.header.description = format!(
                "{} ({}.{}.{})",
                description, major, minor, patch
            );

            match json.dependencies {
                Some(dependencies) => {
                    let mut deps = Vec::<Dependency>::new();
                    for (_, d) in dependencies.clone().iter().enumerate() {
                        if d.module_name.is_some() {
                            deps.push(d.clone().to_owned());
                        } else {
                            let dep = Dependency {
                                uuid: d.uuid.clone(),
                                version: serde_json::to_value(vec![major, minor, patch]).unwrap(),
                                module_name: None
                            };
                            deps.push(dep);
                        }
                    }

                    json.dependencies = Some(deps);
                }
                None => {
                    json.dependencies = Some(Vec::<Dependency>::new());
                }
            }

            buf = serde_json::to_string_pretty(&json).unwrap();

            let manifest_canon_path = path.join("manifest.json").canonicalize().unwrap();

            fs::write(manifest_canon_path, buf).unwrap();
            
            // Create the mcpack
            let p = path.to_str().unwrap();
            match Config::doit(p, &format!("{}.mcpack", p), zip::CompressionMethod::Stored) {
                Ok(_) => {}
                Err(error) => panic!("{}", error.to_string()),
            };
        }

        // Rename the packs with the correct version
        match std::fs::rename(
            &format!("{}.mcpack", &bp_path.to_str().unwrap()),
            format!("{}_bp_{}.mcpack", package_name, &env!("CARGO_PKG_VERSION")),
        ) {
            Ok(_) => {}
            Err(error) => panic!("{}", error.to_string()),
        };
        match std::fs::rename(
            &format!("{}.mcpack", &rp_path.to_str().unwrap()),
            format!("{}_rp_{}.mcpack", package_name, &env!("CARGO_PKG_VERSION")),
        ) {
            Ok(_) => {}
            Err(error) => panic!("{}", error.to_string()),
        };
    }

    fn zip_dir<T>(
        it: &mut dyn Iterator<Item = DirEntry>,
        prefix: &str,
        writer: T,
        method: zip::CompressionMethod,
    ) -> zip::result::ZipResult<()>
    where
        T: Write + Seek,
    {
        let mut zip = zip::ZipWriter::new(writer);
        let options = FileOptions::default()
            .compression_method(method)
            .unix_permissions(0o755);

        let mut buffer = Vec::new();
        for entry in it {
            let path = entry.path();
            let name = path.strip_prefix(Path::new(prefix)).unwrap();

            // Write file or directory explicitly
            // Some unzip tools unzip files with directory paths correctly, some do not!
            if path.is_file() {
                //println!("adding file {path:?} as {name:?} ...");
                #[allow(deprecated)]
                zip.start_file_from_path(name, options)?;
                let mut f = File::open(path)?;

                f.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
                buffer.clear();
            } else if !name.as_os_str().is_empty() {
                // Only if not root! Avoids path spec / warning
                // and mapname conversion failed error on unzip
                //println!("adding dir {path:?} as {name:?} ...");
                #[allow(deprecated)]
                zip.add_directory_from_path(name, options)?;
            }
        }
        zip.finish()?;
        Result::Ok(())
    }

    fn doit(
        src_dir: &str,
        dst_file: &str,
        method: zip::CompressionMethod,
    ) -> zip::result::ZipResult<()> {
        if !Path::new(src_dir).is_dir() {
            return Err(ZipError::FileNotFound);
        }

        let path = Path::new(dst_file);
        let file = File::create(path).unwrap();

        let walkdir = WalkDir::new(src_dir);
        let it = walkdir.into_iter();

        Config::zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method)?;

        Ok(())
    }
}
