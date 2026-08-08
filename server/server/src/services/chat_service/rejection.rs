/// Why a send from the app did not go through.
#[derive(Debug, thiserror::Error)]
pub enum ChatRejection {
    #[error("no chat channel is registered for this world")]
    NoChannel,

    /// The sender is in game somewhere other than the world they addressed — they were
    /// transferred while the app still held the older target. Delivering anyway would put
    /// their message in front of people they are not standing with.
    #[error("sender is not in this world")]
    WrongWorld { current: Option<String> },
}
