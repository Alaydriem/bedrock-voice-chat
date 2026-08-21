use curia::Fields;

use crate::logging::Defect;

use super::{Destination, FieldSpec, RoutedFields};

const RESERVED: &[&str] = &["ts", "level", "target", "msg"];

// AudioDeviceHost's variants are cfg-gated per platform, so the union is
// declared here rather than a per-target set: a Windows user's log file read on
// a Linux triage machine must keep device_host as a tag.
const DECLARED: &[FieldSpec] = &[
    FieldSpec {
        name: "transport",
        destination: Destination::Tag,
        variants: &["quic", "wss"],
    },
    FieldSpec {
        name: "device_host",
        destination: Destination::Tag,
        variants: &["asio", "wasapi", "aaudio", "coreaudio", "alsa", "jack"],
    },
    FieldSpec {
        name: "io",
        destination: Destination::Tag,
        variants: &["input", "output"],
    },
    FieldSpec {
        name: "game",
        destination: Destination::Tag,
        variants: &["minecraft", "bedrock"],
    },
    FieldSpec {
        name: "defect",
        destination: Destination::Tag,
        variants: Defect::NAMES,
    },
    FieldSpec {
        name: "error",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "device_name",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "player_hash",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "connected_server",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "channel_id",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "frames",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "latency_ms",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "seq",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "platform_id",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "install_id",
        destination: Destination::Attribute,
        variants: &[],
    },
    FieldSpec {
        name: "session_id",
        destination: Destination::Attribute,
        variants: &[],
    },
];

pub struct Vocabulary;

impl Vocabulary {
    pub fn declared() -> &'static [FieldSpec] {
        DECLARED
    }

    pub fn is_reserved(key: &str) -> bool {
        RESERVED.contains(&key)
    }

    fn spec(name: &str) -> Option<&'static FieldSpec> {
        DECLARED.iter().find(|f| f.name == name)
    }

    // Tag-destination fields only, for the throttle fingerprint. Attribute
    // fields are excluded deliberately: folding device_name in would defeat the
    // throttle this is meant to sharpen.
    pub fn tag_fields(fields: &Fields) -> Vec<(String, String)> {
        Self::route(fields).tags
    }

    pub fn route(fields: &Fields) -> RoutedFields {
        let mut routed = RoutedFields::default();

        for (key, value) in fields.as_map() {
            match Self::spec(key) {
                Some(spec) => match spec.destination {
                    Destination::Tag => match value.as_str() {
                        Some(s) if spec.variants.contains(&s) => {
                            routed.tags.push((key.clone(), s.to_string()));
                        }
                        // A typo, or a webview caller sending anything it likes.
                        // Searchable as an attribute, never an unbounded tag.
                        _ => {
                            routed.attributes.insert(key.clone(), value.clone());
                        }
                    },
                    Destination::Attribute => {
                        routed.attributes.insert(key.clone(), value.clone());
                    }
                    Destination::Context => {
                        routed.context.insert(key.clone(), value.clone());
                    }
                },
                // An undeclared field is still evidence. Context costs nothing
                // and is never indexed.
                None => {
                    routed.context.insert(key.clone(), value.clone());
                }
            }
        }

        routed
    }
}
