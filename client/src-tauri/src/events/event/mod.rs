pub(crate) mod channel;
pub mod jukebox;
pub(crate) mod notification;
pub mod player_gain_store;
pub(crate) mod player_presence;
pub(crate) mod server_error;

/// One message per finished track during an export. A person watching a gigabyte-sized
/// session render needs to see it moving, and a run that reports only at the end looks
/// stalled for as long as it takes.
pub const RECORDING_EXPORT_PROGRESS: &str = "recording:export-progress";
