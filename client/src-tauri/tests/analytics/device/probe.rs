use std::collections::HashMap;

use bvc_client_lib::analytics::DeviceProbe;

fn lookup(values: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name| map.get(name).cloned()
}

#[test]
fn an_android_market_name_wins_over_the_model_code() {
    let info = DeviceProbe::from_android(
        lookup(&[
            ("ro.product.manufacturer", "OnePlus"),
            ("ro.product.model", "CPH2655"),
            ("ro.vendor.oplus.market.name", "OnePlus 13"),
            ("ro.build.version.release", "16"),
            ("ro.build.version.sdk", "36"),
        ]),
        Some("6.6.89-android15-8".to_string()),
    );

    assert_eq!(info.os_name, "Android");
    assert_eq!(info.os_version.as_deref(), Some("16"));
    assert_eq!(info.os_build.as_deref(), Some("36"));
    assert_eq!(info.manufacturer.as_deref(), Some("OnePlus"));
    assert_eq!(info.model.as_deref(), Some("OnePlus 13"));
    assert_eq!(info.model_id.as_deref(), Some("CPH2655"));
}

#[test]
fn an_android_device_without_a_market_name_reports_its_model() {
    let info = DeviceProbe::from_android(
        lookup(&[
            ("ro.product.manufacturer", "Google"),
            ("ro.product.model", "Pixel 8"),
            ("ro.build.version.release", "14"),
        ]),
        None,
    );

    assert_eq!(info.model.as_deref(), Some("Pixel 8"));
    assert_eq!(info.model_id.as_deref(), Some("Pixel 8"));
    assert_eq!(info.os_version.as_deref(), Some("14"));
}

#[test]
fn an_iphone_reports_ios_and_its_hardware_identifier() {
    let info = DeviceProbe::from_apple(
        lookup(&[
            ("hw.machine", "iPhone17,3"),
            ("hw.model", "D47AP"),
            ("kern.osproductversion", "26.1"),
            ("kern.osversion", "23B85"),
        ]),
        Some("25.1.0".to_string()),
    );

    assert_eq!(info.os_name, "iOS");
    assert_eq!(info.os_version.as_deref(), Some("26.1"));
    assert_eq!(info.os_build.as_deref(), Some("23B85"));
    assert_eq!(info.model.as_deref(), Some("iPhone17,3"));
    assert_eq!(info.model_id.as_deref(), Some("D47AP"));
    assert_eq!(info.family.as_deref(), Some("iPhone"));
    assert_eq!(info.manufacturer.as_deref(), Some("Apple"));
}

#[test]
fn an_ipad_reports_ipados() {
    let info = DeviceProbe::from_apple(lookup(&[("hw.machine", "iPad16,3")]), None);

    assert_eq!(info.os_name, "iPadOS");
    assert_eq!(info.family.as_deref(), Some("iPad"));
}

#[test]
fn desktop_has_no_device_description() {
    assert!(DeviceProbe::current().is_none());
}
