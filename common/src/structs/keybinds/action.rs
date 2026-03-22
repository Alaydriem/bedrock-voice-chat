#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeybindAction {
    ToggleMute,
    ToggleDeafen,
    ToggleRecording,
    PushToTalk,
}
