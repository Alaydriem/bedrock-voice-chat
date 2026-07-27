use bvc_server_lib::config::Database;

fn network_database(scheme: &str) -> Database {
    let mut database = Database::default();
    database.scheme = scheme.to_string();
    database.database = "bvc".to_string();
    database.username = Some("bvc".to_string());
    database.password = Some("hunter2".to_string());
    database.host = Some("db.internal".to_string());
    database
}

#[test]
fn postgres_dsn_uses_postgres_scheme_and_default_port() {
    let database = network_database("postgres");
    assert_eq!(
        database.get_dsn(),
        "postgres://bvc:hunter2@db.internal:5432/bvc"
    );
}

#[test]
fn postgresql_scheme_alias_is_accepted() {
    let database = network_database("postgresql");
    assert!(database.get_dsn().starts_with("postgres://"));
}

#[test]
fn mysql_dsn_uses_default_port_and_host_when_unset() {
    let mut database = network_database("mysql");
    database.host = None;
    assert_eq!(database.get_dsn(), "mysql://bvc:hunter2@127.0.0.1:3306/bvc");
}

#[test]
fn postgres_dsn_appends_tls_params_when_set() {
    let mut database = network_database("postgres");
    database.ssl_mode = Some("verify-full".to_string());
    database.ssl_root_cert = Some("/etc/certs/ca.pem".to_string());
    database.ssl_cert = Some("/etc/certs/client.pem".to_string());
    database.ssl_key = Some("/etc/certs/client.key".to_string());
    assert_eq!(
        database.get_dsn(),
        "postgres://bvc:hunter2@db.internal:5432/bvc\
         ?sslmode=verify-full\
         &sslrootcert=%2Fetc%2Fcerts%2Fca.pem\
         &sslcert=%2Fetc%2Fcerts%2Fclient.pem\
         &sslkey=%2Fetc%2Fcerts%2Fclient.key"
    );
}

#[test]
fn postgres_dsn_omits_unset_tls_params() {
    let mut database = network_database("postgres");
    database.ssl_mode = Some("require".to_string());
    assert_eq!(
        database.get_dsn(),
        "postgres://bvc:hunter2@db.internal:5432/bvc?sslmode=require"
    );
}

#[test]
fn mysql_dsn_translates_ssl_mode_vocabulary() {
    let mut database = network_database("mysql");
    database.ssl_mode = Some("verify-full".to_string());
    database.ssl_root_cert = Some("/etc/certs/ca.pem".to_string());
    assert_eq!(
        database.get_dsn(),
        "mysql://bvc:hunter2@db.internal:3306/bvc\
         ?ssl-mode=verify_identity\
         &ssl-ca=%2Fetc%2Fcerts%2Fca.pem"
    );
}

#[test]
fn credentials_with_reserved_characters_are_percent_encoded() {
    let mut database = network_database("postgres");
    database.username = Some("bvc@prod".to_string());
    database.password = Some("p@ss:w/rd?#".to_string());
    assert_eq!(
        database.get_dsn(),
        "postgres://bvc%40prod:p%40ss%3Aw%2Frd%3F%23@db.internal:5432/bvc"
    );
}

#[test]
fn database_name_cannot_inject_query_parameters() {
    let mut database = network_database("postgres");
    database.database = "bvc?sslmode=disable".to_string();
    database.ssl_mode = Some("verify-full".to_string());
    assert_eq!(
        database.get_dsn(),
        "postgres://bvc:hunter2@db.internal:5432/bvc%3Fsslmode%3Ddisable?sslmode=verify-full"
    );
}

#[test]
fn redacted_dsn_masks_a_configured_password() {
    let database = network_database("postgres");
    let redacted = database.get_redacted_dsn();
    assert_eq!(redacted, "postgres://bvc:***@db.internal:5432/bvc");
    assert!(!redacted.contains("hunter2"));
}

#[test]
fn unknown_scheme_falls_back_to_the_system_sqlite_path() {
    let mut database = network_database("oracle");
    database.ssl_mode = Some("require".to_string());
    assert_eq!(database.get_dsn(), "sqlite:///etc/bvc/bvc.sqlite3");
}

#[test]
fn validate_accepts_a_default_configuration() {
    Database::default().validate().expect("default is valid");
}

#[test]
fn validate_rejects_an_unknown_ssl_mode() {
    let mut database = network_database("postgres");
    database.ssl_mode = Some("mandatory".to_string());
    let err = database.validate().expect_err("mode must be rejected");
    let msg = format!("{err}");
    assert!(msg.contains("mandatory"), "got: {msg}");
    assert!(msg.contains("verify-full"), "got: {msg}");
}

#[test]
fn validate_rejects_a_client_cert_without_a_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cert = dir.path().join("client.pem");
    std::fs::write(&cert, "cert").expect("write cert");

    let mut database = network_database("postgres");
    database.ssl_cert = Some(cert.to_string_lossy().to_string());
    let err = database.validate().expect_err("cert without key");
    assert!(format!("{err}").contains("ssl_key"));
}

#[test]
fn validate_rejects_a_missing_certificate_file() {
    let mut database = network_database("postgres");
    database.ssl_root_cert = Some("/definitely/not/a/real/ca.pem".to_string());
    let err = database.validate().expect_err("missing file");
    assert!(format!("{err}").contains("ssl_root_cert"));
}

#[test]
fn validate_rejects_ssl_options_on_a_non_tls_scheme() {
    let mut database = Database::default();
    database.ssl_mode = Some("require".to_string());
    let err = database.validate().expect_err("sqlite has no TLS");
    assert!(format!("{err}").contains("does not support TLS"));
}

#[test]
fn validate_rejects_tls_material_without_an_ssl_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cert = dir.path().join("client.pem");
    let key = dir.path().join("client.key");
    std::fs::write(&cert, "pem").expect("write cert");
    std::fs::write(&key, "pem").expect("write key");

    let mut database = network_database("postgres");
    database.ssl_cert = Some(cert.to_string_lossy().to_string());
    database.ssl_key = Some(key.to_string_lossy().to_string());
    let err = database.validate().expect_err("mode must be explicit");
    assert!(format!("{err}").contains("ssl_mode must be set"));
}

#[test]
fn validate_rejects_a_non_verifying_mode_with_a_pinned_root_cert() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ca = dir.path().join("ca.pem");
    std::fs::write(&ca, "pem").expect("write ca");

    let mut database = network_database("postgres");
    database.ssl_mode = Some("require".to_string());
    database.ssl_root_cert = Some(ca.to_string_lossy().to_string());
    let err = database.validate().expect_err("require never checks the CA");
    let msg = format!("{err}");
    assert!(msg.contains("verify-ca"), "got: {msg}");
}

#[test]
fn validate_accepts_an_existing_cert_and_key_pair() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ca = dir.path().join("ca.pem");
    let cert = dir.path().join("client.pem");
    let key = dir.path().join("client.key");
    for path in [&ca, &cert, &key] {
        std::fs::write(path, "pem").expect("write pem");
    }

    let mut database = network_database("postgres");
    database.ssl_mode = Some("verify-full".to_string());
    database.ssl_root_cert = Some(ca.to_string_lossy().to_string());
    database.ssl_cert = Some(cert.to_string_lossy().to_string());
    database.ssl_key = Some(key.to_string_lossy().to_string());
    database.validate().expect("full mTLS config is valid");
}
