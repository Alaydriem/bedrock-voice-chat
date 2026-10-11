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
