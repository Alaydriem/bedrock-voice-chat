use std::sync::atomic::{AtomicU32, Ordering};

pub struct PanState {
    left_gain: AtomicU32,
    right_gain: AtomicU32,
    volume: AtomicU32,
}

impl PanState {
    pub fn new() -> Self {
        let equal = 0.5_f32.sqrt();
        Self {
            left_gain: AtomicU32::new(equal.to_bits()),
            right_gain: AtomicU32::new(equal.to_bits()),
            volume: AtomicU32::new(1.0_f32.to_bits()),
        }
    }

    pub fn update(&self, left: f32, right: f32, vol: f32) {
        self.left_gain.store(left.to_bits(), Ordering::Relaxed);
        self.right_gain.store(right.to_bits(), Ordering::Relaxed);
        self.volume.store(vol.to_bits(), Ordering::Relaxed);
    }

    pub fn left_gain(&self) -> f32 {
        f32::from_bits(self.left_gain.load(Ordering::Relaxed))
    }

    pub fn right_gain(&self) -> f32 {
        f32::from_bits(self.right_gain.load(Ordering::Relaxed))
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
}
