/// An Add commit and the Welcome for the member it adds, before the relay has accepted the commit.
#[derive(Debug, Clone)]
pub struct StagedAdd {
    pub commit: Vec<u8>,
    pub welcome: Vec<u8>,
}
