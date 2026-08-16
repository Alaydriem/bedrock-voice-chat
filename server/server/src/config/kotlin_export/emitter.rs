use std::collections::{BTreeMap, BTreeSet};

use anyhow::anyhow;
use schemars::Map;
use schemars::schema::{Schema, SchemaObject};

use super::type_map::KotlinType;
use crate::config::ApplicationConfig;

/// Emits the Java mod's config classes from the server config schema.
pub struct KotlinExporter {
    skip_paths: Vec<String>,
}

impl KotlinExporter {
    pub const PACKAGE: &'static str = "com.alaydriem.bedrockvoicechat.config.generated";

    const ROOT_CLASS: &'static str = "EmbeddedServerConfig";

    // Sections the Java mods deliberately cannot set. Everything else is
    // included by default, so a new server config field reaches the mods
    // without anyone remembering to add it.
    pub const CARVE_OUTS: [&'static str; 4] = [
        "/server/meridian",
        "/server/cors",
        "/server/features/openapi_docs",
        "/server/bedrock/servers",
    ];

    pub fn new() -> Self {
        Self {
            skip_paths: Self::CARVE_OUTS
                .iter()
                .map(|path| path.to_string())
                .collect(),
        }
    }

    pub fn with_skip_paths(skip_paths: Vec<String>) -> Self {
        Self { skip_paths }
    }

    pub fn export(&self) -> Result<BTreeMap<String, String>, anyhow::Error> {
        let root = schemars::schema_for!(ApplicationConfig);
        let mut files = BTreeMap::new();
        let mut resolved_skips = BTreeSet::new();
        let mut visited = BTreeSet::new();

        self.emit_class(
            Self::ROOT_CLASS,
            &root.schema,
            "",
            &root.definitions,
            &mut files,
            &mut resolved_skips,
            &mut visited,
        )?;

        let unresolved: Vec<&str> = self
            .skip_paths
            .iter()
            .filter(|path| !resolved_skips.contains(*path))
            .map(|path| path.as_str())
            .collect();
        if !unresolved.is_empty() {
            return Err(anyhow!(
                "skip paths do not resolve to any config field: {}",
                unresolved.join(", ")
            ));
        }

        Ok(files)
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_class(
        &self,
        class_name: &str,
        schema: &SchemaObject,
        path: &str,
        defs: &Map<String, Schema>,
        files: &mut BTreeMap<String, String>,
        resolved_skips: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), anyhow::Error> {
        if !visited.insert(class_name.to_string()) {
            return Ok(());
        }

        let object = schema
            .object
            .as_ref()
            .ok_or_else(|| anyhow!("{class_name} does not describe an object"))?;

        let mut body = String::new();
        body.push_str(&format!("package {}\n\n", Self::PACKAGE));
        body.push_str("import com.google.gson.annotations.SerializedName\n\n");
        body.push_str("// Generated from the Rust `ApplicationConfig`. Do not edit.\n");
        body.push_str("// Regenerate with:\n");
        body.push_str("//   UPDATE_KOTLIN_CONFIG=1 cargo nextest run kotlin_export\n");
        body.push_str(&format!("class {class_name} {{\n"));

        let mut nested: Vec<(String, SchemaObject, String)> = Vec::new();

        for (field, field_schema) in object.properties.iter() {
            let field_path = format!("{path}/{field}");
            if self.skip_paths.iter().any(|skip| skip == &field_path) {
                resolved_skips.insert(field_path);
                continue;
            }

            let kotlin_type = KotlinType::of(field_schema, defs)
                .map_err(|e| anyhow!("{class_name}.{field}: {e}"))?;

            body.push_str(&format!("    @SerializedName(\"{field}\")\n"));
            body.push_str(&format!(
                "    var {}: {}? = null\n\n",
                Self::camel_case(field),
                kotlin_type
            ));

            if let Some((name, definition)) = Self::referenced_definition(field_schema, defs)? {
                nested.push((name, definition, field_path));
            }
        }

        body.push_str("}\n");
        files.insert(format!("{class_name}.kt"), body);

        for (name, definition, field_path) in nested {
            self.emit_class(
                &name,
                &definition,
                &field_path,
                defs,
                files,
                resolved_skips,
                visited,
            )?;
        }

        Ok(())
    }

    // Follows a `$ref`, including one wrapped in the anyOf/null shape an
    // Option<T> produces and one held as a map's value type, and returns the
    // definition it names.
    //
    // The map case is not cosmetic: `KotlinType` renders a `HashMap<String, T>`
    // as `Map<String, T>` whether or not `T`'s class is ever emitted, so without
    // this the generated Kotlin references a type that does not exist.
    fn referenced_definition(
        schema: &Schema,
        defs: &Map<String, Schema>,
    ) -> Result<Option<(String, SchemaObject)>, anyhow::Error> {
        let object = match schema {
            Schema::Object(object) => object,
            Schema::Bool(_) => return Ok(None),
        };

        let map_value_reference = || -> Option<String> {
            match object
                .object
                .as_ref()
                .and_then(|validation| validation.additional_properties.as_ref())
                .map(|values| values.as_ref())
            {
                Some(Schema::Object(values)) => values.reference.clone(),
                _ => None,
            }
        };

        let reference = match &object.reference {
            Some(reference) => Some(reference.clone()),
            None => match KotlinType::first_non_null_subschema(object) {
                Some(Schema::Object(inner)) => inner.reference.clone(),
                _ => map_value_reference(),
            },
        };

        let Some(reference) = reference else {
            return Ok(None);
        };

        let name = reference
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("malformed reference {reference}"))?
            .to_string();
        let definition = defs
            .get(&name)
            .ok_or_else(|| anyhow!("reference {reference} has no definition"))?;

        // A named definition holding a primitive maps to that primitive, so
        // there is no class to emit for it.
        if !KotlinType::describes_an_object(definition) {
            return Ok(None);
        }

        Ok(Some((name, definition.clone().into_object())))
    }

    fn camel_case(field: &str) -> String {
        let mut out = String::with_capacity(field.len());
        let mut capitalize = false;
        for character in field.chars() {
            if character == '_' {
                capitalize = true;
                continue;
            }
            if capitalize {
                out.extend(character.to_uppercase());
                capitalize = false;
            } else {
                out.push(character);
            }
        }
        out
    }
}

impl Default for KotlinExporter {
    fn default() -> Self {
        Self::new()
    }
}
