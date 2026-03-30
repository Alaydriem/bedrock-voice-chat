use okapi::openapi3::{OpenApi, Tag};

/// A tag description that is auto-collected via `inventory`.
/// Submit from any route module to register a tag in the OpenAPI spec.
pub struct TagDefinition {
    pub name: &'static str,
    pub description: &'static str,
}

inventory::collect!(TagDefinition);

/// A route group spec that is auto-collected via `inventory`.
/// Submit from any route module to register routes in the OpenAPI spec.
pub struct RouteSpec {
    pub prefix: &'static str,
    pub spec_fn: fn() -> (Vec<rocket::Route>, OpenApi),
}

inventory::collect!(RouteSpec);

pub struct OpenApiSpec;

impl OpenApiSpec {
    pub fn generate() -> OpenApi {
        let mut merged = OpenApi {
            openapi: "3.0.0".to_string(),
            info: okapi::openapi3::Info {
                title: "Bedrock Voice Chat Server API".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: Some("Bedrock Voice Chat Server HTTP API".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        for route_spec in inventory::iter::<RouteSpec> {
            let (_, spec) = (route_spec.spec_fn)();
            if let Err(e) = okapi::merge::merge_specs(&mut merged, &route_spec.prefix, &spec) {
                tracing::error!("Failed to merge OpenAPI spec for {}: {}", route_spec.prefix, e);
            }
        }

        merged.tags = inventory::iter::<TagDefinition>
            .into_iter()
            .map(|td| Tag {
                name: td.name.to_string(),
                description: Some(td.description.to_string()),
                ..Default::default()
            })
            .collect();

        merged
    }

    pub fn routes() -> Vec<(&'static str, Vec<rocket::Route>)> {
        inventory::iter::<RouteSpec>
            .into_iter()
            .map(|rs| {
                let (routes, _) = (rs.spec_fn)();
                (rs.prefix, routes)
            })
            .collect()
    }
}
