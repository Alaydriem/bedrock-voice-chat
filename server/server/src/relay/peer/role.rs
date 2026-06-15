// Per-link topology role. Mesh is the only role consulted by routing; Hub/Spoke
// carry no routing behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRole {
    Mesh,
    Hub,
    Spoke,
}

impl Default for PeerRole {
    fn default() -> Self {
        Self::Mesh
    }
}

// Advisory capacity descriptor. `cores` is the machine's reported parallelism;
// `open_peers` is the remaining peer-connection headroom. Not consulted by
// routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    pub cores: usize,
    pub open_peers: usize,
}

impl Caps {
    pub fn new(cores: usize, open_peers: usize) -> Self {
        Self { cores, open_peers }
    }

    // Detects local core count from `available_parallelism`, falling back to 1
    // when the platform cannot report it.
    pub fn detect(open_peers: usize) -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self { cores, open_peers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_defaults_to_mesh() {
        assert_eq!(PeerRole::default(), PeerRole::Mesh);
    }

    #[test]
    fn caps_detect_reports_at_least_one_core() {
        let caps = Caps::detect(3);
        assert!(caps.cores >= 1);
        assert_eq!(caps.open_peers, 3);
    }
}
