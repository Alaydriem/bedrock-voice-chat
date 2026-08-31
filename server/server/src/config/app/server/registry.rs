// Where this server reaches the BVC registry.
//
// Two independent features dial it: enrollment, which is for entitled members, and
// address observation, which is for every server because peering is not a paid feature.
// It therefore belongs to neither of their config blocks.
//
// Essential infrastructure and opaque. Baked into the binary at build time from
// `.env.local` and readable nowhere else: not a config key, not a runtime environment
// variable. An operator never writes it, and a build that shipped without one cannot be
// pointed at a registry after the fact — it has to be rebuilt.
//
// That is deliberate. A runtime override is a way to aim somebody else's server at a
// registry the operator did not choose, and the value is not a secret worth protecting
// so much as a decision that belongs to whoever produced the binary.
pub struct Registry;

impl Registry {
    // `option_env!` resolves against the environment rustc was invoked with, which is
    // why `build.rs` loads `.env.local` and re-emits this as `cargo:rustc-env`. Reading
    // the file here instead would find nothing: the file is not present at runtime, and
    // by then the decision is long since made.
    pub fn peerlink() -> Option<String> {
        Self::sanitize(option_env!("BVC_REGISTRY_PEERLINK"))
    }

    // A blank bake is no bake. An empty `BVC_REGISTRY_PEERLINK=` in `.env.local` reads
    // to `option_env!` as `Some("")`, and carrying that forward would fail at the dial
    // with a parse error rather than at the point where the answer is "this build has
    // no registry".
    pub fn sanitize(raw: Option<&str>) -> Option<String> {
        raw.map(str::trim)
            .filter(|link| !link.is_empty())
            .map(str::to_string)
    }
}
