// The spatial settings actually in force for this session, as resolved from the server-supplied
// metadata rather than the compiled defaults.
//
// Fields are `pub(super)` because `SessionConfig` reads them directly to answer its accessors; no
// consumer outside that module sees this type.
#[derive(Debug, Clone)]
pub(super) struct Spatial {
    pub(super) proximity_range: f32,
    pub(super) falloff: String,
}
