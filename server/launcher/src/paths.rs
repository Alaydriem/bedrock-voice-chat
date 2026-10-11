use std::path::{Path, PathBuf};

/// Every path the launcher reads or writes, relative to the BDS directory.
#[derive(Debug, Clone)]
pub struct LauncherPaths {
    root: PathBuf,
}

impl LauncherPaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn bvc_dir(&self) -> PathBuf {
        self.root.join("bvc")
    }

    pub fn config_file(&self) -> PathBuf {
        self.bvc_dir().join("config.hcl")
    }

    pub fn bvc_binary(&self) -> PathBuf {
        self.bvc_dir().join("bin").join("bvc-server")
    }

    pub fn version_file(&self) -> PathBuf {
        self.bvc_dir().join("bin").join("VERSION")
    }

    pub fn pack_file(&self) -> PathBuf {
        self.bvc_dir().join("packs").join("bvc-bds-pack.mcaddon")
    }

    pub fn behavior_packs_dir(&self) -> PathBuf {
        self.root.join("development_behavior_packs")
    }

    pub fn resource_packs_dir(&self) -> PathBuf {
        self.root.join("development_resource_packs")
    }

    pub fn permissions_file(&self) -> PathBuf {
        self.root.join("config").join("default").join("permissions.json")
    }

    pub fn variables_file(&self) -> PathBuf {
        self.root.join("config").join("default").join("variables.json")
    }

    pub fn server_properties(&self) -> PathBuf {
        self.root.join("server.properties")
    }

    pub fn bds_binary(&self) -> PathBuf {
        self.root.join("bedrock_server_vanilla")
    }

    pub fn level_dat(&self, level_name: &str) -> PathBuf {
        self.root.join("worlds").join(level_name).join("level.dat")
    }
}
