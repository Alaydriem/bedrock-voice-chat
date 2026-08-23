//! Page size clamping for the admin roster.
//!
//! The clamp is the only thing between one admin request and a response carrying every
//! player row a server has ever registered.

use bvc_server_lib::services::AdminUserService;

#[test]
fn absent_page_size_uses_the_default() {
    assert_eq!(AdminUserService::clamp_page_size(None), 20);
}

#[test]
fn a_page_size_above_the_maximum_is_capped() {
    assert_eq!(AdminUserService::clamp_page_size(Some(5_000)), 100);
}

#[test]
fn zero_is_not_a_page_size() {
    assert_eq!(AdminUserService::clamp_page_size(Some(0)), 20);
}

#[test]
fn a_page_size_within_range_is_honoured() {
    assert_eq!(AdminUserService::clamp_page_size(Some(8)), 8);
}
