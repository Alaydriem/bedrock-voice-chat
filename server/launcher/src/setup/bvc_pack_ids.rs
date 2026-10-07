/// Header UUIDs of every BVC pack a world may already list: the full variant the launcher
/// installs and the no-net variant, whose pack file the install removes.
pub struct BvcPackIds;

impl BvcPackIds {
    pub const BEHAVIOR: [&'static str; 2] = [
        "6fb24263-357a-407c-aebe-681f50b2de50",
        "508a43ed-b95e-40cf-98eb-d78f2d72caa4",
    ];
    pub const RESOURCE: [&'static str; 2] = [
        "20a36865-747e-4c38-8077-ea9228c2bd5d",
        "9a656a2e-defd-4f17-8b9d-011414c8abff",
    ];
}
