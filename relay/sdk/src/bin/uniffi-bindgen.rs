// uniffi ships its bindgen as a binary of the `uniffi` crate, which cargo will
// not run from a dependency. This is the documented way to get one: a binary in
// the crate being bound, delegating to the same entry point.
fn main() {
    uniffi::uniffi_bindgen_main()
}
