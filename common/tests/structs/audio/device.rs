use common::structs::audio::{AudioDevice, AudioDeviceHost, AudioDeviceType, StreamConfig};

fn config(sample_rate: u32) -> StreamConfig {
    StreamConfig {
        channels: 1,
        sample_rate,
        sample_format: String::from("f32"),
        buffer_size_min: 0,
        buffer_size_max: 0,
    }
}

fn device(display_name: &str, id: &str, sample_rate: u32) -> AudioDevice {
    AudioDevice {
        io: AudioDeviceType::InputDevice,
        id: String::from(id),
        name: String::from(display_name),
        host: AudioDeviceHost::default(),
        stream_configs: vec![config(sample_rate)],
        display_name: String::from(display_name),
    }
}

fn names(devices: &[AudioDevice]) -> Vec<String> {
    devices.iter().map(|d| d.display_name.clone()).collect()
}

fn rates(devices: &[AudioDevice]) -> Vec<u32> {
    devices
        .iter()
        .map(|d| d.stream_configs[0].sample_rate)
        .collect()
}

/// The ASIO enumeration emits one entry per supported config, all named from the channel
/// count alone, so a driver offering 48 kHz and 44.1 kHz on one channel produced two
/// entries a user could not tell apart and the picker could not key on.
#[test]
fn one_entry_survives_per_device_and_name() {
    let deduplicated = AudioDevice::deduplicate(vec![
        device("Focusrite USB ASIO Input 1", "focusrite", 44100),
        device("Focusrite USB ASIO Input 1", "focusrite", 48000),
    ]);

    assert_eq!(names(&deduplicated), vec!["Focusrite USB ASIO Input 1"]);
}

#[test]
fn the_highest_sample_rate_is_the_one_kept() {
    let deduplicated = AudioDevice::deduplicate(vec![
        device("Focusrite USB ASIO Input 1", "focusrite", 44100),
        device("Focusrite USB ASIO Input 1", "focusrite", 48000),
    ]);

    assert_eq!(rates(&deduplicated), vec![48000]);
}

/// The rate decides which entry wins, not which entry arrived first.
#[test]
fn the_highest_sample_rate_wins_from_either_position() {
    let deduplicated = AudioDevice::deduplicate(vec![
        device("Focusrite USB ASIO Input 1", "focusrite", 48000),
        device("Focusrite USB ASIO Input 1", "focusrite", 44100),
    ]);

    assert_eq!(rates(&deduplicated), vec![48000]);
}

/// One ASIO device carries every channel as its own entry under a single device id.
/// Collapsing on the id alone would offer channel 1 and silently discard the rest.
#[test]
fn channels_of_one_device_are_not_collapsed_into_each_other() {
    let deduplicated = AudioDevice::deduplicate(vec![
        device("Focusrite USB ASIO Input 1", "focusrite", 48000),
        device("Focusrite USB ASIO Input 2", "focusrite", 48000),
    ]);

    assert_eq!(
        names(&deduplicated),
        vec![
            "Focusrite USB ASIO Input 1",
            "Focusrite USB ASIO Input 2"
        ]
    );
}

/// One interface is both a capture and a playback endpoint, and the two lists are built
/// from one enumeration pass. Collapsing across the direction would lose one of them.
#[test]
fn the_capture_and_playback_sides_of_one_name_are_two_devices() {
    let input = device("Focusrite USB", "focusrite", 48000);
    let mut output = device("Focusrite USB", "focusrite", 48000);
    output.io = AudioDeviceType::OutputDevice;

    let deduplicated = AudioDevice::deduplicate(vec![input, output]);

    assert_eq!(deduplicated.len(), 2);
}

/// The picker renders the list in the order it is given, so a device must not move
/// because a duplicate of something above it was dropped.
#[test]
fn the_surviving_order_is_the_order_it_arrived_in() {
    let deduplicated = AudioDevice::deduplicate(vec![
        device("Blue Yeti", "yeti", 48000),
        device("Focusrite USB ASIO Input 1", "focusrite", 44100),
        device("Realtek", "realtek", 48000),
        device("Focusrite USB ASIO Input 1", "focusrite", 48000),
    ]);

    assert_eq!(
        names(&deduplicated),
        vec!["Blue Yeti", "Focusrite USB ASIO Input 1", "Realtek"]
    );
}

/// A device with no usable config is filtered out before this, but the ranking must not
/// panic if one ever reaches it.
#[test]
fn a_device_without_a_config_is_still_returned() {
    let mut bare = device("Bare", "bare", 48000);
    bare.stream_configs = vec![];

    let deduplicated = AudioDevice::deduplicate(vec![bare]);

    assert_eq!(names(&deduplicated), vec!["Bare"]);
}

/// A config list is ranked by its best rate, not by whichever happens to be first.
#[test]
fn an_entry_is_ranked_by_its_highest_rate() {
    let mut lower = device("Multi", "multi", 44100);
    lower.stream_configs = vec![config(44100)];
    let mut higher = device("Multi", "multi", 8000);
    higher.stream_configs = vec![config(8000), config(48000)];

    let deduplicated = AudioDevice::deduplicate(vec![lower, higher]);

    assert_eq!(deduplicated.len(), 1);
    assert_eq!(deduplicated[0].stream_configs.len(), 2);
}
