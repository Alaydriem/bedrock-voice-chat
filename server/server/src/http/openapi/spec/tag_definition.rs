/// A tag description that is auto-collected via `inventory`.
/// Submit from any route module to register a tag in the OpenAPI spec.
pub struct TagDefinition {
    pub name: &'static str,
    pub description: &'static str,
}

inventory::collect!(TagDefinition);
