use clap::Parser;
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;

use super::Command;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run<'a>(&'a self, _cfg: &Arc<Command>) {
        //let localappdata = match env::var("LOCALAPPDATA") {
        let localappdata = match env::var("UserProfile") {
            Ok(v) => v,
            Err(_) => panic!("Missing LOCALAPPDATA"),
        };

        let bp_path = format!("{}/Projects/Alaydriem/bedrock-server-1.20.41.02/development_behavior_packs/bedrock-voice-chat", localappdata);
        let rp_path = format!("{}/Projects/Alaydriem/bedrock-server-1.20.41.02/development_resource_packs/bedrock-voice-chat", localappdata);
        //let bp_path = format!("{}/Packages/Microsoft.MinecraftUWP_8wekyb3d8bbwe/LocalState/games/com.mojang/development_behavior_packs/creator_challenge", localappdata);
        //let rp_path = format!("{}/Packages/Microsoft.MinecraftUWP_8wekyb3d8bbwe/LocalState/games/com.mojang/development_resource_packs/creator_challenge", localappdata);
        
        let mut paths = HashMap::<String, String>::new();
        paths.insert(bp_path, String::from("pack/bp"));
        paths.insert(rp_path, String::from("pack/rp"));
        for (dst, _) in &paths {
            // Remove the current project directory
            match std::fs::remove_dir_all(&dst) {
                Ok(_) => {}
                Err(_) => {}
            };

            // Create the directory structure
            match std::fs::create_dir_all(&dst) {
                Ok(_) => {}
                Err(error) => panic!("{}", error.to_string()),
            };
        }

        let options = fs_extra::dir::CopyOptions::new(); //Initialize default values for CopyOptions
        let handle = |process_info: fs_extra::dir::TransitProcess|  {
            println!("{}", process_info.total_bytes);
            fs_extra::dir::TransitProcessResult::ContinueOrAbort
        };

        for (src, dst) in &paths {
            fs_extra::dir::copy_with_progress(src, dst, &options, handle);
        }

        let work_path = Path::new("pack");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).unwrap();
        match watcher.watch(work_path.as_ref(), RecursiveMode::Recursive) {
            Ok(_) => {}
            Err(error) => panic!("{}", error.to_string()),
        };

        // let canonical_bp_path = std::fs::canonicalize("pack/bp").unwrap();
        // let canonical_rp_path = std::fs::canonicalize("pack/rp").unwrap();
        for res in rx {
            match res {
                Ok(event) => {
                    if event.kind.is_create() || event.kind.is_modify() || event.kind.is_remove() {
                        // debounce would be nice -- log what triggered the change
                        println!("{:?}", event.paths);
                        // Ineffecient but ensures syncroniztion...
                        for (dst, src) in &paths {
                            // Remove the directory entirely
                            match std::fs::remove_dir_all(&dst) {
                                Ok(_) => {}
                                Err(_) => {}
                            };

                            // Copy it back over
                            match copy_dir::copy_dir(&src, &dst) {
                                Ok(_) => {}
                                Err(error) => {
                                    panic!("{}", error.to_string())
                                }
                            };
                        }
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }
}
