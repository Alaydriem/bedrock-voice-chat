use bvc_client_lib::keybinds::PttHold;

#[test]
fn the_first_press_is_the_one_that_opens_the_mic() {
    let hold = PttHold::new();
    assert!(hold.press());
    assert!(hold.is_held());
}

/// A held hotkey repeats. Each repeat is a press, and treating every one as new would
/// re-run the open — and, paired with a release each, would chatter the microphone.
#[test]
fn a_repeat_press_is_not_a_second_press() {
    let hold = PttHold::new();
    assert!(hold.press());
    assert!(!hold.press());
    assert!(!hold.press());
    assert!(hold.is_held());
}

/// The bug this exists to prevent.
///
/// A tap in push-to-talk sends a press and a release. On a phone whose backend had not yet
/// been told the mode, the press was refused and the release was not — so a tap closed a
/// microphone it had never opened, and the meter went flat with no way to bring it back.
#[test]
fn a_release_with_no_press_behind_it_does_nothing() {
    let hold = PttHold::new();
    assert!(!hold.release());
    assert!(!hold.is_held());
}

#[test]
fn a_release_after_a_press_is_the_one_that_closes() {
    let hold = PttHold::new();
    hold.press();
    assert!(hold.release());
    assert!(!hold.is_held());
    // And only once: a second release is unpaired like any other.
    assert!(!hold.release());
}

/// The tail keeps the microphone open for a beat after the release. A press inside that
/// window is someone carrying on talking, so the pending close has to stand down.
#[test]
fn a_press_during_the_tail_keeps_the_mic_open() {
    let hold = PttHold::new();
    hold.press();
    hold.release();
    assert!(hold.tail_should_close());

    hold.press();
    assert!(!hold.tail_should_close());
}

#[test]
fn a_tail_with_no_new_press_closes() {
    let hold = PttHold::new();
    hold.press();
    hold.release();
    assert!(hold.tail_should_close());
}

/// A voice-mode change resets the microphone on its own. A hold surviving one would leave a
/// stale claim, and the next release would be unpaired.
#[test]
fn clearing_forgets_a_hold() {
    let hold = PttHold::new();
    hold.press();
    hold.clear();

    assert!(!hold.is_held());
    assert!(!hold.release());
    assert!(hold.press());
}

/// Clones share the flag: the release tail runs on a spawned task holding its own handle,
/// and it has to see a press that arrived after it started waiting.
#[test]
fn a_clone_watches_the_same_hold() {
    let hold = PttHold::new();
    let tail = hold.clone();

    hold.press();
    assert!(tail.is_held());
    assert!(!tail.tail_should_close());
}
