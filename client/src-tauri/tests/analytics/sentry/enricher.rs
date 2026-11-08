use std::time::SystemTime;

use bvc_client_lib::PlatformId;
use bvc_client_lib::analytics::sentry::SentryEnricher;
use common::structs::app::DeviceInfo;
use sentry::protocol::{Context, DeviceContext, Event, Log, LogAttribute, LogLevel, OsContext};

fn android() -> DeviceInfo {
    DeviceInfo {
        os_name: "Android".to_string(),
        os_version: Some("16".to_string()),
        os_build: Some("36".to_string()),
        kernel_version: None,
        manufacturer: Some("OnePlus".to_string()),
        brand: Some("OnePlus".to_string()),
        model: Some("OnePlus 13".to_string()),
        model_id: Some("CPH2655".to_string()),
        family: None,
    }
}

fn log() -> Log {
    Log {
        level: LogLevel::Warn,
        body: "capture died".to_string(),
        trace_id: None,
        timestamp: SystemTime::now(),
        severity_number: None,
        attributes: Default::default(),
    }
}

fn attribute(log: &Log, key: &str) -> Option<String> {
    log.attributes
        .get(key)
        .and_then(|LogAttribute(value)| value.as_str().map(String::from))
}

#[test]
fn a_refreshed_platform_id_reaches_the_next_event() {
    let platform_id = PlatformId::new_shared("first".to_string());
    let enricher = SentryEnricher::new(platform_id.clone(), None);

    platform_id.set("second".to_string());
    let event = enricher.enrich_event(Event::default());

    assert_eq!(event.user.and_then(|u| u.id).as_deref(), Some("second"));
}

#[test]
fn no_user_is_reported_before_the_platform_id_is_known() {
    let enricher = SentryEnricher::new(PlatformId::new_shared(String::new()), None);

    assert!(enricher.enrich_event(Event::default()).user.is_none());
    assert!(enricher.enrich_log(log()).attributes.get("user.id").is_none());
}

#[test]
fn a_mobile_device_replaces_the_kernel_derived_os_and_keeps_the_arch() {
    let enricher = SentryEnricher::new(PlatformId::new_shared("id".to_string()), Some(android()));

    let mut event = Event::default();
    event.contexts.insert(
        "os".to_string(),
        OsContext {
            name: Some("Linux".to_string()),
            version: Some("6.12.58-android16-6".to_string()),
            kernel_version: Some("#1 SMP PREEMPT".to_string()),
            ..Default::default()
        }
        .into(),
    );
    event.contexts.insert(
        "device".to_string(),
        DeviceContext {
            arch: Some("aarch64".to_string()),
            ..Default::default()
        }
        .into(),
    );

    let event = enricher.enrich_event(event);

    let Some(Context::Os(os)) = event.contexts.get("os") else {
        panic!("os context missing");
    };
    assert_eq!(os.name.as_deref(), Some("Android"));
    assert_eq!(os.version.as_deref(), Some("16"));
    assert_eq!(os.kernel_version.as_deref(), Some("#1 SMP PREEMPT"));

    let Some(Context::Device(device)) = event.contexts.get("device") else {
        panic!("device context missing");
    };
    assert_eq!(device.model.as_deref(), Some("OnePlus 13"));
    assert_eq!(device.model_id.as_deref(), Some("CPH2655"));
    assert_eq!(device.arch.as_deref(), Some("aarch64"));
    assert_eq!(device.other.get("manufacturer"), Some(&"OnePlus".into()));
}

#[test]
fn a_mobile_log_overrides_the_cached_os_name() {
    let enricher = SentryEnricher::new(PlatformId::new_shared("id".to_string()), Some(android()));

    let mut entry = log();
    entry
        .attributes
        .insert("os.name".to_string(), LogAttribute("Linux".into()));

    let entry = enricher.enrich_log(entry);

    assert_eq!(attribute(&entry, "user.id").as_deref(), Some("id"));
    assert_eq!(attribute(&entry, "os.name").as_deref(), Some("Android"));
    assert_eq!(attribute(&entry, "os.version").as_deref(), Some("16"));
    assert_eq!(attribute(&entry, "device.manufacturer").as_deref(), Some("OnePlus"));
    assert_eq!(attribute(&entry, "device.model").as_deref(), Some("OnePlus 13"));
}

#[test]
fn desktop_keeps_what_the_sdk_detected() {
    let enricher = SentryEnricher::new(PlatformId::new_shared("id".to_string()), None);

    let mut entry = log();
    entry
        .attributes
        .insert("os.name".to_string(), LogAttribute("windows".into()));

    let entry = enricher.enrich_log(entry);

    assert_eq!(attribute(&entry, "os.name").as_deref(), Some("windows"));
    assert!(entry.attributes.get("device.model").is_none());
}
