/// Which of the two Rocket listeners a `RocketManager` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RocketListener {
    /// The TLS listener with the whole API, behind the demultiplexer's public port.
    Public,
    /// The plain-HTTP listener for the game mods on this machine. It has no certificate,
    /// mounts only `LoopbackRoutes`, and is reachable only through the demultiplexer's
    /// loopback branch.
    ModLoopback,
}
