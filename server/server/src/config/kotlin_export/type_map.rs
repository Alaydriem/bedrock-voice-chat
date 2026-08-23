use anyhow::anyhow;
use schemars::Map;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};

/// Translates a JSON schema node into the Kotlin type that represents it.
pub struct KotlinType;

impl KotlinType {
    pub fn of(schema: &Schema, defs: &Map<String, Schema>) -> Result<String, anyhow::Error> {
        match schema {
            Schema::Object(object) => Self::of_object(object, defs),
            Schema::Bool(_) => Err(anyhow!("a boolean schema describes no Kotlin type")),
        }
    }

    fn of_object(
        object: &SchemaObject,
        defs: &Map<String, Schema>,
    ) -> Result<String, anyhow::Error> {
        if let Some(reference) = &object.reference {
            let name = Self::class_name(reference);
            // A type that describes itself as a primitive still earns a named
            // definition, so a reference is only a Kotlin class when the thing
            // it points at actually has fields.
            return match defs.get(&name) {
                Some(definition) if !Self::describes_an_object(definition) => {
                    Self::of(definition, defs)
                }
                _ => Ok(name),
            };
        }

        if let Some(inner) = Self::first_non_null_subschema(object) {
            return Self::of(inner, defs);
        }

        let instance_type = object
            .instance_type
            .as_ref()
            .ok_or_else(|| anyhow!("schema has no instance type and no reference"))?;

        match Self::primary_instance(instance_type)? {
            InstanceType::String => Ok("String".to_string()),
            InstanceType::Boolean => Ok("Boolean".to_string()),
            InstanceType::Integer => Ok(Self::integer_type(object.format.as_deref())),
            InstanceType::Number => Ok(Self::number_type(object.format.as_deref())),
            InstanceType::Array => {
                let items = object
                    .array
                    .as_ref()
                    .and_then(|array| array.items.as_ref())
                    .ok_or_else(|| anyhow!("array schema has no items"))?;
                let item = match items {
                    SingleOrVec::Single(schema) => schema.as_ref(),
                    SingleOrVec::Vec(schemas) => schemas
                        .first()
                        .ok_or_else(|| anyhow!("array schema has an empty items list"))?,
                };
                Ok(format!("List<{}>", Self::of(item, defs)?))
            }
            InstanceType::Object => {
                let values = object
                    .object
                    .as_ref()
                    .and_then(|validation| validation.additional_properties.as_ref())
                    .ok_or_else(|| {
                        anyhow!("an inline object without additionalProperties has no Kotlin type")
                    })?;
                Ok(format!("Map<String, {}>", Self::of(values, defs)?))
            }
            InstanceType::Null => Err(anyhow!("a null-only schema describes no Kotlin type")),
        }
    }

    // Every unsigned type wider than 16 bits maps to Long: u32 exceeds
    // Int.MAX_VALUE, and a narrower mapping would truncate rather than fail.
    fn integer_type(format: Option<&str>) -> String {
        match format {
            Some("int8") | Some("uint8") | Some("int16") | Some("uint16") | Some("int32") => "Int",
            _ => "Long",
        }
        .to_string()
    }

    fn number_type(format: Option<&str>) -> String {
        match format {
            Some("float") => "Float",
            _ => "Double",
        }
        .to_string()
    }

    /// Whether a schema carries named fields, as opposed to describing a
    /// primitive or a map.
    pub(super) fn describes_an_object(schema: &Schema) -> bool {
        match schema {
            Schema::Object(object) => object
                .object
                .as_ref()
                .map(|validation| !validation.properties.is_empty())
                .unwrap_or(false),
            Schema::Bool(_) => false,
        }
    }

    /// The first meaningful branch of a composed schema.
    ///
    /// An `Option<T>` over a struct describes as `anyOf [T, null]`, and a field
    /// carrying a `serde(default)` describes as `allOf [T]` with the default
    /// alongside — the reference is one level down in both shapes.
    pub(super) fn first_non_null_subschema(object: &SchemaObject) -> Option<&Schema> {
        let subschemas = object.subschemas.as_ref()?;
        [
            subschemas.all_of.as_ref(),
            subschemas.any_of.as_ref(),
            subschemas.one_of.as_ref(),
        ]
        .into_iter()
        .flatten()
        .flat_map(|branches| branches.iter())
        .find(|schema| !Self::is_null(schema))
    }

    fn class_name(reference: &str) -> String {
        reference.rsplit('/').next().unwrap_or(reference).to_string()
    }

    fn is_null(schema: &Schema) -> bool {
        match schema {
            Schema::Object(object) => match &object.instance_type {
                Some(SingleOrVec::Single(kind)) => **kind == InstanceType::Null,
                Some(SingleOrVec::Vec(kinds)) => {
                    kinds.len() == 1 && kinds[0] == InstanceType::Null
                }
                None => false,
            },
            Schema::Bool(_) => false,
        }
    }

    fn primary_instance(
        instance_type: &SingleOrVec<InstanceType>,
    ) -> Result<InstanceType, anyhow::Error> {
        match instance_type {
            SingleOrVec::Single(kind) => Ok(**kind),
            SingleOrVec::Vec(kinds) => kinds
                .iter()
                .find(|kind| **kind != InstanceType::Null)
                .copied()
                .ok_or_else(|| anyhow!("schema is null-only")),
        }
    }
}
