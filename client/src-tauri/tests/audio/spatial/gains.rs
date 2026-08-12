use bvc_client_lib::audio::spatial::SpatialGains;

// Equal power, so a voice moving across the stereo field does not change loudness.
#[test]
fn a_pan_pair_sums_to_unit_power() {
    for pan in [-1.0, -0.5, 0.0, 0.35, 1.0] {
        let gains = SpatialGains::from_pan(pan, 1.0, 1.0);
        let power = gains.left * gains.left + gains.right * gains.right;

        assert!(
            (power - 1.0).abs() < 1e-5,
            "pan {} gave power {}",
            pan,
            power
        );
    }
}

#[test]
fn centred_puts_equal_gain_in_both_ears_at_unity_volume() {
    let gains = SpatialGains::centred();

    assert_eq!(gains.left, gains.right);
    assert_eq!(gains.volume, 1.0);
    assert!((gains.left - 0.5f32.sqrt()).abs() < 1e-6);
}

#[test]
fn a_full_left_pan_puts_everything_in_the_left() {
    let gains = SpatialGains::from_pan(1.0, 1.0, 1.0);

    assert!((gains.left - 1.0).abs() < 1e-6);
    assert!(gains.right.abs() < 1e-6);
}

#[test]
fn a_full_right_pan_puts_everything_in_the_right() {
    let gains = SpatialGains::from_pan(-1.0, 1.0, 1.0);

    assert!((gains.right - 1.0).abs() < 1e-6);
    assert!(gains.left.abs() < 1e-6);
}

// The listener's panning intensity pulls a pan back toward centre without touching volume.
#[test]
fn panning_intensity_narrows_the_field() {
    let full = SpatialGains::from_pan(1.0, 1.0, 1.0);
    let narrowed = SpatialGains::from_pan(1.0, 1.0, 0.5);

    assert!(narrowed.left < full.left);
    assert!(narrowed.right > full.right);
    assert_eq!(narrowed.volume, full.volume);
}

#[test]
fn a_zero_panning_intensity_is_centred_whatever_the_geometry_says() {
    let gains = SpatialGains::from_pan(1.0, 1.0, 0.0);

    assert!((gains.left - gains.right).abs() < 1e-6);
}

#[test]
fn a_pan_scaled_past_full_scale_is_clamped() {
    let clamped = SpatialGains::from_pan(4.0, 1.0, 1.0);
    let full = SpatialGains::from_pan(1.0, 1.0, 1.0);

    assert!((clamped.left - full.left).abs() < 1e-6);
    assert!((clamped.right - full.right).abs() < 1e-6);
}

#[test]
fn volume_passes_through_untouched() {
    let gains = SpatialGains::from_pan(0.25, 0.4, 0.8);

    assert_eq!(gains.volume, 0.4);
}

// The pair used to be computed inline in the playback sink. This reproduces that arithmetic and
// holds the extracted version to it bit for bit, so the move out of the audio hot path cannot
// have changed a single sample anyone hears.
#[test]
fn the_extracted_pair_matches_the_arithmetic_it_replaced() {
    for pan in [-1.0f32, -0.8, -0.3, 0.0, 0.15, 0.6, 1.0] {
        for intensity in [0.0f32, 0.25, 0.8, 1.0] {
            let scaled = (pan * intensity).clamp(-1.0, 1.0);
            let expected_left = ((1.0 + scaled) / 2.0).sqrt();
            let expected_right = ((1.0 - scaled) / 2.0).sqrt();

            let gains = SpatialGains::from_pan(pan, 0.75, intensity);

            assert_eq!(
                gains.left, expected_left,
                "left differs at pan {} intensity {}",
                pan, intensity
            );
            assert_eq!(
                gains.right, expected_right,
                "right differs at pan {} intensity {}",
                pan, intensity
            );
        }
    }
}
