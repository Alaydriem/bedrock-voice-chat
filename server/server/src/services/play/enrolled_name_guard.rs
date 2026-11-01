use anyhow::{Result, anyhow};

/// Refuses play mode on a server that holds a name from token enrollment.
///
/// An enrolled name switches the server to the registry's ACME provider, and the registry
/// serves a node with a registration through the enrolled path, so a play name would get no
/// DNS record. Running both is worse than running neither, so startup stops.
pub struct EnrolledNameGuard;

impl EnrolledNameGuard {
    pub fn check(play_enabled: bool, enrolled_name: Option<&str>) -> Result<()> {
        match (play_enabled, enrolled_name) {
            (true, Some(name)) => Err(anyhow!(
                "tls.play and an enrolled name are mutually exclusive: this server is enrolled \
                 as {name}. Remove tls.play to keep that name. To use play mode instead, delete \
                 the stored enrolled name (the row named relay_assigned_name in the server_secret \
                 table of this server's database). The enrolled name is then released and cannot \
                 be reissued, so back the database up first."
            )),
            _ => Ok(()),
        }
    }
}
