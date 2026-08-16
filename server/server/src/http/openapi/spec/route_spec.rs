use okapi::openapi3::OpenApi;

/// A route group spec that is auto-collected via `inventory`.
/// Submit from any route module to register routes in the OpenAPI spec.
pub struct RouteSpec {
    pub prefix: &'static str,
    // When false, the group contributes to the generated OpenAPI spec but is NOT
    // auto-mounted by `routes()`. Used for routes mounted manually under runtime
    // conditions (e.g. feature-gated relay routes) that must still be documented.
    pub auto_mount: bool,
    pub spec_fn: fn() -> (Vec<rocket::Route>, OpenApi),
}

inventory::collect!(RouteSpec);
