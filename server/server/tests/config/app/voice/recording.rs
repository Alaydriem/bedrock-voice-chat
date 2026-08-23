use bvc_server_lib::config::ApplicationConfig;

// A document with no recording block at all. Absence must read as permitted:
// a server upgrading into this feature keeps working for the creators on it.
const NO_RECORDING_BLOCK: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
}
"#;

// The assertion that matters. A bool whose default is true is easy to write in a
// way where an explicit false never reaches the config, and the absent-vs-true
// case cannot tell the difference.
const RECORDING_DISABLED: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
}
voice {
    recording {
        enabled = false
    }
}
"#;

const RECORDING_ENABLED: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
}
voice {
    recording {
        enabled = true
    }
}
"#;

#[test]
fn absent_recording_block_permits_recording() {
    let cfg = ApplicationConfig::from_hcl_str_with_env(NO_RECORDING_BLOCK, &Default::default())
        .expect("document without a recording block must parse");
    assert!(
        cfg.voice.recording.enabled,
        "a server that never configured recording must keep permitting it"
    );
}

#[test]
fn explicit_false_disables_recording() {
    let cfg = ApplicationConfig::from_hcl_str_with_env(RECORDING_DISABLED, &Default::default())
        .expect("document must parse");
    assert!(
        !cfg.voice.recording.enabled,
        "enabled = false must reach the config"
    );
}

#[test]
fn explicit_true_permits_recording() {
    let cfg = ApplicationConfig::from_hcl_str_with_env(RECORDING_ENABLED, &Default::default())
        .expect("document must parse");
    assert!(cfg.voice.recording.enabled);
}

// `voice` is a sibling of `server`, not a child of it. Nested inside `server` the block
// deserializes into nothing — no error, no warning, and recording stays enabled.
const RECORDING_NESTED_UNDER_SERVER: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
    voice {
        recording {
            enabled = false
        }
    }
}
"#;

#[test]
fn a_recording_block_nested_under_server_does_not_disable_recording() {
    let cfg =
        ApplicationConfig::from_hcl_str_with_env(RECORDING_NESTED_UNDER_SERVER, &Default::default())
            .expect("document parses");
    assert!(
        cfg.voice.recording.enabled,
        "misplacing the block must be visible here rather than silently taking effect"
    );
}
