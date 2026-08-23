/// Owns a hardware handle whose `drop` is the device release.
///
/// Generic over the handle because the ordering is the logic and the payload is not.
pub struct DeviceLease<T: Send + 'static> {
    held: Option<T>,
}

impl<T: Send + 'static> DeviceLease<T> {
    pub fn empty() -> Self {
        Self { held: None }
    }

    pub fn is_held(&self) -> bool {
        self.held.is_some()
    }

    /// Takes a new handle, releasing any handle it displaces before returning.
    pub async fn hold(&mut self, handle: Option<T>) {
        let displaced = std::mem::replace(&mut self.held, handle);
        Self::give_back(displaced).await;
    }

    /// Returns once the held handle has been dropped, or immediately if there is none.
    pub async fn release(&mut self) {
        Self::give_back(self.held.take()).await;
    }

    /// On the blocking pool: a driver can spend real time inside `drop`, and callers are on the
    /// runtime the rest of the stop path runs on.
    async fn give_back(handle: Option<T>) {
        let Some(handle) = handle else {
            return;
        };

        if let Err(e) = tokio::task::spawn_blocking(move || drop(handle)).await {
            log::warn!("Device release did not complete cleanly: {:?}", e);
        }
    }
}

impl<T: Send + 'static> Default for DeviceLease<T> {
    fn default() -> Self {
        Self::empty()
    }
}
