use bvc_server_lib::config::ApplicationConfig;
use bvc_server_lib::runtime::ServerRuntime;
use common::curia::Level;

const QUIET: &str = "hyper=off,rustls=off,rocket::server=off,rocket_http::tls::listener=off,metrics_exporter_dogstatsd::forwarder=off";

#[test]
fn the_default_directives_silence_the_noisy_targets_at_info() {
    assert_eq!(
        ServerRuntime::default_directives(Level::Info),
        format!("info,{QUIET}")
    );
}

#[test]
fn the_default_directives_silence_the_noisy_targets_at_warn_and_error() {
    assert_eq!(
        ServerRuntime::default_directives(Level::Warn),
        format!("warn,{QUIET}")
    );
    assert_eq!(
        ServerRuntime::default_directives(Level::Error),
        format!("error,{QUIET}")
    );
}

#[test]
fn the_default_directives_at_debug_keep_only_the_tls_listener_silenced() {
    assert_eq!(
        ServerRuntime::default_directives(Level::Debug),
        "info,rocket_http::tls::listener=off"
    );
}

#[test]
fn the_default_directives_at_trace_are_a_bare_debug_floor() {
    assert_eq!(ServerRuntime::default_directives(Level::Trace), "debug");
}

#[test]
fn logging_init_is_idempotent_within_a_process() {
    let mut first = ServerRuntime::new(ApplicationConfig::default()).unwrap();
    let mut second = ServerRuntime::new(ApplicationConfig::default()).unwrap();

    first.setup_logging().unwrap();
    second.setup_logging().unwrap();
}

#[test]
fn an_unwritable_log_path_still_starts_the_server() {
    // A path whose parent is a regular file can never be created as a
    // directory, on any platform. A hardcoded root path is not portable: the
    // drive root is writable for some Windows accounts and not others.
    let blocker = tempfile::NamedTempFile::new().unwrap();
    let unwritable = blocker.path().join("logs");

    let mut config = ApplicationConfig::default();
    config.log.path = unwritable.to_string_lossy().to_string();

    let mut runtime = ServerRuntime::new(config).unwrap();

    assert!(
        runtime.setup_logging().is_ok(),
        "an unwritable log directory must degrade to console-only, not abort startup"
    );
}
