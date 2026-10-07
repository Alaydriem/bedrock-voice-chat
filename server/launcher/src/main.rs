use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use bvc_launcher::Launcher;
use bvc_launcher::release::ReleaseSources;

fn main() {
    // The directory holding the launcher is the BDS directory, wherever the panel starts it from.
    let root = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let bds_args: Vec<OsString> = env::args_os().skip(1).collect();
    std::process::exit(Launcher::new(root, bds_args, ReleaseSources::default()).run());
}
