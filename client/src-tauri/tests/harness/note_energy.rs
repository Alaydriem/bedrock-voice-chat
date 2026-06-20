use bvc_client_lib::testkit::signal::Signal;

/// Goertzel note-energy assertions over a 48 kHz mono capture.
pub struct NoteEnergy;

impl NoteEnergy {
    /// True when every note in `notes` carries > 2% Goertzel energy in the capture.
    pub fn all_present(mono: &[f32], notes: &[f32]) -> bool {
        notes
            .iter()
            .all(|&f| Signal::tone_energy_fraction(mono, 48_000, f) > 0.02)
    }

    /// True when NO note in `notes` carries >= 1% energy (absent / cross-bleed-free).
    pub fn all_absent(mono: &[f32], notes: &[f32]) -> bool {
        notes
            .iter()
            .all(|&f| Signal::tone_energy_fraction(mono, 48_000, f) < 0.01)
    }
}
