use bvc_server_lib::services::play::EnrolledNameGuard;

// Reviewed defect I2: an enrolled server that switched to play ran ACME beside it, and its
// play name never got a DNS record.
#[test]
fn play_on_an_enrolled_server_is_refused_with_the_way_out() {
    let error = EnrolledNameGuard::check(true, Some("creeper-diorite-badlands.bedrockvc.stream"))
        .expect_err("play and an enrolled name cannot share a server");
    let message = error.to_string();

    assert!(message.contains("tls.play and an enrolled name are mutually exclusive"));
    assert!(message.contains("creeper-diorite-badlands.bedrockvc.stream"));
    assert!(message.contains("relay_assigned_name"));
}

#[test]
fn an_enrolled_server_without_play_is_untouched() {
    assert!(EnrolledNameGuard::check(false, Some("a.bedrockvc.stream")).is_ok());
}

#[test]
fn play_without_an_enrolled_name_is_allowed() {
    assert!(EnrolledNameGuard::check(true, None).is_ok());
}
