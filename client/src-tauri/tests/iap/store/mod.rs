// `StoreProvider` is re-exported from the crate root only under `e2e`, so this
// module compiles solely in the e2e configuration.
#[cfg(feature = "e2e")]
mod provider;
