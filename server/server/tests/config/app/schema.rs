use bvc_server_lib::config::ApplicationConfig;

// The generator resolves carve-outs and field types through the schema, so the
// shape it depends on is asserted here rather than inside the generator.
#[test]
fn application_config_schema_exposes_every_section() {
    let root = schemars::schema_for!(ApplicationConfig);
    let object = root
        .schema
        .object
        .as_ref()
        .expect("root schema is an object");

    for section in ["database", "server", "log", "voice", "audio", "permissions"] {
        assert!(
            object.properties.contains_key(section),
            "root schema is missing the {section:?} section"
        );
    }

    for definition in [
        "Server",
        "Tls",
        "Acme",
        "Minecraft",
        "Features",
        "BedrockConfig",
        "Age",
        "Database",
        "Logger",
        "Voice",
        "SpatialAudioConfig",
        "Audio",
        "Permissions",
        "PeerConfig",
    ] {
        assert!(
            root.definitions.contains_key(definition),
            "schema definitions are missing {definition:?}"
        );
    }
}

// AcmeProviderKind round-trips through String, so it must describe as one. It
// still earns its own definition, and the string-ness lives there rather than
// on the field, which only holds the reference.
#[test]
fn acme_provider_kind_is_a_string_in_the_schema() {
    let root = schemars::schema_for!(ApplicationConfig);
    let provider = root
        .definitions
        .get("AcmeProviderKind")
        .cloned()
        .expect("AcmeProviderKind has a definition")
        .into_object();

    assert!(
        provider.object.is_none(),
        "AcmeProviderKind must not describe named fields"
    );

    let rendered = serde_json::to_string(&provider).expect("provider schema serializes");
    assert!(
        rendered.contains("string"),
        "AcmeProviderKind must describe as a string, got {rendered}"
    );
}
