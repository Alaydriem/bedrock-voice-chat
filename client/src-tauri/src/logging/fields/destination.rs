#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Tag,
    Attribute,
    Context,
}
