use bvc_launcher::config::BvcConfig;

#[test]
fn server_url_prefers_the_first_acme_domain() {
    let config = BvcConfig::from_hcl_str(
        r#"
        server {
            tls {
                names = ["fallback.example.com"]
                acme {
                    domains = ["voice.example.com", "other.example.com"]
                }
            }
        }
        "#,
    )
    .unwrap();

    assert_eq!(config.server_url().as_deref(), Some("https://voice.example.com"));
}

#[test]
fn server_url_falls_back_to_the_first_tls_name() {
    let config = BvcConfig::from_hcl_str(
        r#"
        server {
            tls {
                names = ["voice.example.com"]
                acme {
                    domains = []
                }
            }
        }
        "#,
    )
    .unwrap();

    assert_eq!(config.server_url().as_deref(), Some("https://voice.example.com"));
}

#[test]
fn server_url_includes_a_port_other_than_443() {
    let config = BvcConfig::from_hcl_str(
        r#"
        server {
            port = 3000
            tls {
                names = ["voice.example.com"]
            }
        }
        "#,
    )
    .unwrap();

    assert_eq!(config.server_url().as_deref(), Some("https://voice.example.com:3000"));
}

#[test]
fn server_url_is_none_without_any_name() {
    let config = BvcConfig::from_hcl_str("server {}").unwrap();

    assert_eq!(config.server_url(), None);
}

#[test]
fn a_config_with_acme_is_not_play_mode_and_keeps_its_https_url() {
    let config = BvcConfig::from_hcl_str(
        r#"
        server {
            tls {
                acme {
                    domains = ["voice.example.com"]
                }
            }
        }
        "#,
    )
    .unwrap();

    assert!(!config.is_play_mode());
    assert!(!config.names_no_certificate_source(None));
    assert_eq!(config.server_url().as_deref(), Some("https://voice.example.com"));
}

#[test]
fn play_mode_with_loopback_posts_to_loopback() {
    let config = BvcConfig::from_hcl_str(
        "server {\n  port = 3000\n  mod_loopback = true\n  tls {\n    play {}\n  }\n}\n",
    )
    .unwrap();

    assert!(config.is_play_mode());
    assert_eq!(config.server_url().as_deref(), Some("http://127.0.0.1:3000"));
}

#[test]
fn play_mode_without_loopback_does_not_post_to_a_dead_loopback_url() {
    let config = BvcConfig::from_hcl_str(
        "server {\n  port = 3000\n  tls {\n    names = [\"voice.example.com\"]\n    play {}\n  }\n}\n",
    )
    .unwrap();

    assert!(config.is_play_mode());
    assert_eq!(config.server_url().as_deref(), Some("https://voice.example.com:3000"));
}

#[test]
fn a_config_naming_no_certificate_source_is_warned_about_once_with_both_lines() {
    let config = BvcConfig::from_hcl_str("server {\n  tls {\n    names = [\"a.example.com\"]\n  }\n}\n")
        .unwrap();

    let warning = config.certificate_source_warning(None).expect("a warning");
    assert!(warning.contains("mod_loopback = true"));
    assert!(warning.contains("play {}"));
}

#[test]
fn a_config_with_a_certificate_source_is_not_warned_about() {
    let sources = [
        "server {\n  enrollment {\n    token = \"abc\"\n  }\n}\n",
        "server {\n  tls {\n    certificate = \"c.pem\"\n  }\n}\n",
        "server {\n  tls {\n    play {}\n  }\n}\n",
        "server {\n  tls {\n    acme {\n      domains = [\"a.example.com\"]\n    }\n  }\n}\n",
    ];
    for hcl in sources {
        let config = BvcConfig::from_hcl_str(hcl).unwrap();
        assert_eq!(config.certificate_source_warning(None), None, "{hcl}");
    }
}

#[test]
fn an_enrollment_token_in_the_environment_counts_as_a_certificate_source() {
    let config = BvcConfig::from_hcl_str("server {}").unwrap();

    assert_eq!(config.certificate_source_warning(Some("tok")), None);
    assert!(config.certificate_source_warning(Some("  ")).is_some());
    assert!(config.certificate_source_warning(None).is_some());
}

#[test]
fn blank_values_do_not_count_as_a_certificate_source() {
    let blank_certificate =
        BvcConfig::from_hcl_str("server {\n  tls {\n    certificate = \"\"\n  }\n}\n").unwrap();
    let blank_token =
        BvcConfig::from_hcl_str("server {\n  enrollment {\n    token = \" \"\n  }\n}\n").unwrap();

    assert!(blank_certificate.names_no_certificate_source(None));
    assert!(blank_token.names_no_certificate_source(None));
}

#[test]
fn unrelated_blocks_are_ignored() {
    let config = BvcConfig::from_hcl_str(
        r#"
        server {
            tls {
                names = ["voice.example.com"]
            }
            bedrock {
                servers = [{ name = "SMP", host = "a.example.com", port = 19132 }]
            }
        }
        voice {
            recording {
                enabled = true
            }
        }
        "#,
    )
    .unwrap();

    assert_eq!(config.server_url().as_deref(), Some("https://voice.example.com"));
}

#[test]
fn an_unset_environment_reference_is_an_error() {
    let result = BvcConfig::from_hcl_str(
        r#"
        server {
            tls {
                names = ["${env.BVC_LAUNCHER_TEST_UNSET_VARIABLE}"]
            }
        }
        "#,
    );

    assert!(result.is_err());
}
