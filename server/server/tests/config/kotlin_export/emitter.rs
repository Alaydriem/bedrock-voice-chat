use bvc_server_lib::config::KotlinExporter;

#[test]
fn emits_one_file_per_config_class() {
    let files = KotlinExporter::new().export().expect("export succeeds");

    for name in [
        "EmbeddedServerConfig.kt",
        "Server.kt",
        "Tls.kt",
        "Acme.kt",
        "Minecraft.kt",
        "Features.kt",
        "RelayFeature.kt",
        "BedrockConfig.kt",
        "Age.kt",
        "Database.kt",
        "Logger.kt",
        "Voice.kt",
        "SpatialAudioConfig.kt",
        "Audio.kt",
        "Permissions.kt",
    ] {
        assert!(files.contains_key(name), "expected {name} in the export");
    }
}

#[test]
fn the_root_class_is_renamed_and_carries_every_section() {
    let files = KotlinExporter::new().export().expect("export succeeds");
    let root = files
        .get("EmbeddedServerConfig.kt")
        .expect("root class is emitted");

    assert!(root.contains("class EmbeddedServerConfig"));
    assert!(root.contains("package com.alaydriem.bedrockvoicechat.config.generated"));
    for section in ["database", "server", "log", "voice", "audio", "permissions"] {
        assert!(
            root.contains(&format!("@SerializedName(\"{section}\")")),
            "root class is missing the {section} section"
        );
    }
}

// Kotlin properties are camelCase; the wire name is whatever serde uses.
#[test]
fn properties_are_camel_case_with_the_serde_name_annotated() {
    let files = KotlinExporter::new().export().expect("export succeeds");
    let tls = files.get("Tls.kt").expect("Tls is emitted");

    assert!(tls.contains("@SerializedName(\"certs_path\")"));
    assert!(tls.contains("var certsPath: String? = null"));
}

// A type that serializes as a string still earns its own schema definition.
// Following that reference blindly would emit a Kotlin class an operator could
// never write in JSON.
#[test]
fn a_string_backed_enum_maps_to_string_rather_than_a_class() {
    let files = KotlinExporter::new().export().expect("export succeeds");
    let acme = files.get("Acme.kt").expect("Acme is emitted");

    assert!(acme.contains("var provider: String? = null"));
    assert!(
        !files.contains_key("AcmeProviderKind.kt"),
        "a string-backed enum must not become a class"
    );
}

// Nothing is copied from Rust: an unset key must be absent from the JSON so the
// server's own serde default applies.
#[test]
fn every_property_is_nullable_and_defaults_to_null() {
    let files = KotlinExporter::new().export().expect("export succeeds");

    for (name, body) in files.iter() {
        for line in body
            .lines()
            .filter(|line| line.trim_start().starts_with("var "))
        {
            assert!(
                line.trim_end().ends_with("? = null"),
                "{name} has a non-nullable property: {line}"
            );
        }
    }
}

#[test]
fn carved_out_sections_are_absent() {
    let files = KotlinExporter::new().export().expect("export succeeds");

    let server = files.get("Server.kt").expect("Server is emitted");
    assert!(
        !server.contains("meridian"),
        "server.meridian must be carved out"
    );
    assert!(!server.contains("cors"), "server.cors must be carved out");
    assert!(
        !files.contains_key("Meridian.kt"),
        "Meridian must not be emitted"
    );
    assert!(!files.contains_key("Cors.kt"), "Cors must not be emitted");

    let features = files.get("Features.kt").expect("Features is emitted");
    assert!(
        !features.contains("openapi"),
        "features.openapi_docs must be carved out"
    );

    let bedrock = files
        .get("BedrockConfig.kt")
        .expect("BedrockConfig is emitted");
    assert!(
        !bedrock.contains("servers"),
        "bedrock.servers must be carved out"
    );
    assert!(
        !files.contains_key("BedrockServerEntry.kt"),
        "BedrockServerEntry must not be emitted"
    );
}

// A renamed Rust field would leave its skip path dangling and silently publish a
// carved-out section to the mods. That must fail loudly instead.
#[test]
fn an_unresolvable_skip_path_is_an_error() {
    let exporter = KotlinExporter::with_skip_paths(vec!["/server/not_a_real_field".to_string()]);
    let result = exporter.export();

    let message = result
        .err()
        .expect("unresolvable skip path errors")
        .to_string();
    assert!(
        message.contains("/server/not_a_real_field"),
        "the error must name the unresolved path, got: {message}"
    );
}

#[test]
fn generated_files_carry_a_do_not_edit_header() {
    let files = KotlinExporter::new().export().expect("export succeeds");
    for (name, body) in files.iter() {
        assert!(
            body.contains("Do not edit"),
            "{name} is missing the generated-file header"
        );
    }
}
