use common::bedrock_protocol::version::ProtocolVersion;

pub struct ProtocolMatrix;

impl ProtocolMatrix {
    // How many released protocols the proxy scenarios cover.
    const RELEASED_DEPTH: usize = 2;

    /// The protocols every proxy scenario runs against: the newest released ones, plus the
    /// preview ahead of them.
    ///
    /// Derived, never hardcoded. A break on a released protocol is a live regression; the
    /// preview leg is the early warning, and reports nothing about anything shipped.
    pub fn coverage() -> Vec<ProtocolVersion> {
        let mut versions = Self::released();
        versions.extend(Self::next_preview());
        versions
    }

    /// The newest released protocols, oldest first.
    ///
    /// Two filters rather than one. `GENERATED_ALL` interleaves preview codecs with released
    /// ones, so its tail is not the newest releases; and a preview can sit *below*
    /// `RELEASED_LATEST` — V2187 does — so a version test alone does not exclude one either.
    fn released() -> Vec<ProtocolVersion> {
        let released: Vec<ProtocolVersion> = ProtocolVersion::GENERATED_ALL
            .into_iter()
            .filter(|v| *v <= ProtocolVersion::RELEASED_LATEST)
            .filter(|v| !ProtocolVersion::GENERATED_PREVIEW.contains(v))
            .collect();

        released[released.len().saturating_sub(Self::RELEASED_DEPTH)..].to_vec()
    }

    /// The newest preview ahead of the released line, where the crate carries one.
    ///
    /// The newest rather than the oldest: previews supersede each other, so the last one is what
    /// the next release is being cut from. An already-superseded preview breaking says nothing
    /// about what is coming.
    fn next_preview() -> Option<ProtocolVersion> {
        ProtocolVersion::GENERATED_PREVIEW
            .into_iter()
            .filter(|v| *v > ProtocolVersion::RELEASED_LATEST)
            .last()
    }
}

// Coverage has to be two released protocols plus the preview ahead of them. A released protocol
// is what players actually speak, so a break there is a live regression; the preview is the early
// warning, and on its own it reports nothing about anything shipped.
#[test]
fn coverage_is_two_released_protocols_and_one_preview() {
    let covered = ProtocolMatrix::coverage();

    let released: Vec<ProtocolVersion> = covered
        .iter()
        .copied()
        .filter(|v| !ProtocolVersion::GENERATED_PREVIEW.contains(v))
        .collect();
    let preview: Vec<ProtocolVersion> = covered
        .iter()
        .copied()
        .filter(|v| ProtocolVersion::GENERATED_PREVIEW.contains(v))
        .collect();

    assert_eq!(
        released.len(),
        2,
        "expected two released protocols, got {released:?} out of {covered:?}; \
         GENERATED_ALL carries preview codecs interleaved with released ones, so its tail is \
         not the newest releases"
    );
    assert_eq!(
        preview.len(),
        1,
        "expected exactly one preview, got {preview:?} out of {covered:?}"
    );
    assert!(
        preview[0] > ProtocolVersion::RELEASED_LATEST,
        "the covered preview {:?} is not ahead of RELEASED_LATEST {:?}, so it warns of nothing",
        preview[0],
        ProtocolVersion::RELEASED_LATEST
    );
}

// The preview leg only warns of the release being cut next. Picking the oldest preview above the
// line would pin coverage to something already superseded.
#[test]
fn the_preview_leg_is_the_newest_one_ahead_of_the_released_line() {
    let newest = ProtocolVersion::GENERATED_PREVIEW
        .into_iter()
        .filter(|v| *v > ProtocolVersion::RELEASED_LATEST)
        .last()
        .expect("the crate carries a preview ahead of RELEASED_LATEST");

    assert_eq!(ProtocolMatrix::coverage().last().copied(), Some(newest));
}
