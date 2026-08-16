#[derive(Default)]
pub(super) struct DeviceState {
    pub(super) attempts: u32,
    pub(super) open: bool,
}
