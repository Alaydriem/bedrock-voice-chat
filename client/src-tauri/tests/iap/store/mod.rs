// `StoreProvider` is re-exported from the crate root only under `e2e` with `iap`,
// so this module compiles solely in that configuration.
#[cfg(all(feature = "e2e", feature = "iap"))]
mod provider;
