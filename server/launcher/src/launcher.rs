use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Console;
use crate::LauncherPaths;
use crate::config::BvcConfig;
use crate::release::{ReleaseDownloader, ReleaseSources};
use crate::setup::{
    BvcPackIds, InstalledPacks, PackInstaller, PermissionsFile, ServerProperties, VariablesFile,
    WorldPacksFile, WorldPatcher,
};
use crate::token::TokenMinter;

/// The replacement `bedrock_server`: sets up the server and world, then runs BVC and BDS.
pub struct Launcher {
    paths: LauncherPaths,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    bds_args: Vec<OsString>,
    sources: ReleaseSources,
}

impl Launcher {
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    const FIRST_WORLD_WAIT: Duration = Duration::from_secs(15);
    const MINT_ATTEMPTS: u32 = 30;
    const MINT_DELAY: Duration = Duration::from_secs(1);

    pub fn new(root: PathBuf, bds_args: Vec<OsString>, sources: ReleaseSources) -> Self {
        Self {
            paths: LauncherPaths::new(root),
            bds_args,
            sources,
        }
    }

    pub fn run(&self) -> i32 {
        let config = self.paths.config_file();
        if !config.exists() {
            Console::error(&format!(
                "{} not found. Create it, then start the server again. BDS and BVC were not started.",
                config.display()
            ));
            return 1;
        }

        self.update();
        let packs = self.install_packs();
        self.ensure_permissions();
        let world_exists = self.prepare_world(packs.as_ref());
        self.supervise(world_exists, packs.as_ref())
    }

    fn update(&self) {
        let result = ReleaseDownloader::new(self.paths.clone(), self.sources.clone())
            .and_then(|d| d.update());
        if let Err(e) = result {
            Console::warn(&format!("Update skipped, using the files on disk: {e:#}"));
        }
    }

    fn install_packs(&self) -> Option<InstalledPacks> {
        let pack = self.paths.pack_file();
        if !pack.exists() {
            Console::warn("No BVC add-on on disk; packs not installed");
            return None;
        }
        let installer =
            PackInstaller::new(self.paths.behavior_packs_dir(), self.paths.resource_packs_dir());
        match installer.install(&pack) {
            Ok(installed) => Some(installed),
            Err(e) => {
                Console::warn(&format!("Packs not installed: {e:#}"));
                None
            }
        }
    }

    fn ensure_permissions(&self) {
        let permissions = PermissionsFile::new(self.paths.permissions_file());
        if let Err(e) = permissions.ensure_required_modules() {
            Console::warn(&format!("permissions.json not updated: {e:#}"));
        }
    }

    /// Enables Beta APIs and the installed packs on the world BDS loads. Returns whether the
    /// world exists; a world BDS has not created yet is prepared after its first start.
    fn prepare_world(&self, packs: Option<&InstalledPacks>) -> bool {
        let level_name = ServerProperties::level_name(&self.paths.server_properties());
        let level_dat = self.paths.level_dat(&level_name);
        if !level_dat.exists() {
            return false;
        }
        match WorldPatcher::enable_beta_apis(&level_dat) {
            Ok(true) => Console::info(&format!("Enabled Beta APIs for {level_name}")),
            Ok(false) => {}
            Err(e) => Console::warn(&format!("Beta APIs not enabled: {e:#}")),
        }
        if let (Some(packs), Some(world_dir)) = (packs, level_dat.parent()) {
            Self::register_packs(world_dir, packs);
        }
        true
    }

    fn register_packs(world_dir: &Path, packs: &InstalledPacks) {
        let registrations = [
            ("world_behavior_packs.json", &packs.behavior, &BvcPackIds::BEHAVIOR),
            ("world_resource_packs.json", &packs.resource, &BvcPackIds::RESOURCE),
        ];
        for (file, pack, replaces) in registrations {
            let world_packs = WorldPacksFile::new(world_dir.join(file));
            match world_packs.register(pack, replaces) {
                Ok(true) => Console::info(&format!(
                    "Enabled pack {} {:?} in {file}",
                    pack.uuid, pack.version
                )),
                Ok(false) => {}
                Err(e) => Console::warn(&format!("{file} not updated: {e:#}")),
            }
        }
    }

    /// A token is minted only from a running BVC: a BVC that failed to start would only hold BDS
    /// back for the whole retry window.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    fn write_variables(&self, bvc_running: bool) {
        let path = self.paths.variables_file();
        let mut variables = match VariablesFile::load(&path) {
            Ok(variables) => variables,
            Err(e) => {
                Console::warn(&format!("variables.json not updated: {e:#}"));
                return;
            }
        };

        let mut changed = false;
        if variables.is_missing(VariablesFile::SERVER_KEY) {
            match BvcConfig::load(&self.paths.config_file()).map(|c| c.server_url()) {
                Ok(Some(url)) => {
                    variables.set(VariablesFile::SERVER_KEY, &url);
                    changed = true;
                }
                Ok(None) => Console::warn(
                    "config.hcl names no server.tls.acme.domains or server.tls.names; bvc_server not set",
                ),
                Err(e) => Console::warn(&format!("bvc_server not set: {e:#}")),
            }
        }

        if variables.is_missing(VariablesFile::TOKEN_KEY) && bvc_running {
            let minter = TokenMinter::new(
                self.paths.bvc_binary(),
                self.paths.bvc_dir(),
                Self::MINT_ATTEMPTS,
                Self::MINT_DELAY,
            );
            match minter.mint() {
                Ok(token) => {
                    variables.set(VariablesFile::TOKEN_KEY, &token);
                    changed = true;
                }
                Err(e) => Console::warn(&format!("bvc_access_token not set: {e:#}")),
            }
        }

        if changed && let Err(e) = variables.save() {
            Console::warn(&format!("variables.json not saved: {e:#}"));
        }
    }

    #[cfg(target_os = "linux")]
    fn supervise(&self, world_exists: bool, packs: Option<&InstalledPacks>) -> i32 {
        use std::sync::mpsc;

        use crate::process::{
            BdsInput, ChildLock, IdleOutcome, Reaper, SignalListener, StdinForwarder, Supervisor,
        };

        let (events, receiver) = mpsc::channel();
        if let Err(e) = SignalListener::spawn(events.clone()) {
            Console::warn(&format!("Signal handling unavailable: {e:#}"));
        }
        let bds_input = BdsInput::new();
        let child_lock = ChildLock::new();
        let mut supervisor = Supervisor::new(receiver, bds_input.clone(), child_lock.clone());

        let bvc_running = if !self.paths.bvc_binary().exists() {
            Console::warn("No BVC server on disk; starting BDS without voice");
            false
        } else if let Err(e) =
            supervisor.spawn_bvc(&self.paths.bvc_binary(), &self.paths.bvc_dir())
        {
            Console::warn(&format!("BVC not started: {e:#}"));
            false
        } else {
            true
        };

        // The minter waits on its own child, so it must finish before the Reaper's waitpid(-1)
        // starts collecting every child.
        self.write_variables(bvc_running);
        Reaper::spawn(events, child_lock);
        StdinForwarder::spawn(bds_input);

        if let Some(code) = self.start_bds(&mut supervisor) {
            return code;
        }

        if !world_exists {
            match supervisor.idle(Self::FIRST_WORLD_WAIT) {
                IdleOutcome::Signalled => return supervisor.shutdown(),
                IdleOutcome::BdsExited(code) => {
                    supervisor.stop_bvc();
                    return code;
                }
                IdleOutcome::Elapsed => {
                    Console::info("Restarting BDS to enable Beta APIs on the new world");
                    if !supervisor.restart_bds_stop() {
                        return supervisor.shutdown();
                    }
                    if !self.prepare_world(packs) {
                        Console::warn(
                            "The world was not created; Beta APIs and packs not enabled",
                        );
                    }
                    if let Some(code) = self.start_bds(&mut supervisor) {
                        return code;
                    }
                }
            }
        }

        supervisor.run()
    }

    /// Starts BDS unless a stop was requested first. Returns the launcher's exit code when it
    /// must exit instead.
    #[cfg(target_os = "linux")]
    fn start_bds(&self, supervisor: &mut crate::process::Supervisor) -> Option<i32> {
        if supervisor.signal_pending() {
            Console::info("Stop requested before BDS started; stopping BVC");
            supervisor.stop_bvc();
            return Some(0);
        }
        if let Err(e) =
            supervisor.spawn_bds(&self.paths.bds_binary(), self.paths.root(), &self.bds_args)
        {
            Console::error(&format!("BDS not started: {e:#}"));
            supervisor.stop_bvc();
            return Some(1);
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    fn supervise(&self, _world_exists: bool, _packs: Option<&InstalledPacks>) -> i32 {
        Console::error("The BVC launcher runs on Linux only");
        1
    }
}
