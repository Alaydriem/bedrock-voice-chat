use std::time::{SystemTime, UNIX_EPOCH};

mod update;

pub use update::ActivityUpdate;


pub struct ActivityDetector {
    rms_threshold: f32,
    activity_tx: Option<flume::Sender<ActivityUpdate>>,
    sample_accumulator: Vec<f32>,
    samples_per_analysis: usize,
    player_name: String,
    last_emission_time: u64,
    emission_cooldown_ms: u64,
}

impl ActivityDetector {
    pub fn new(
        player_name: String,
        sample_rate: u32,
        activity_tx: Option<flume::Sender<ActivityUpdate>>,
    ) -> Self {
        // Analyze every ~10ms worth of samples for responsiveness
        // A 10 ms analysis window, floored so a very low sample rate still yields
        // enough samples for a meaningful RMS.
        let samples_per_analysis = (sample_rate as f32 * 0.01) as usize;
        let samples_per_analysis = samples_per_analysis.max(32);

        Self {
            rms_threshold: 0.01,
            activity_tx,
            sample_accumulator: Vec::with_capacity(samples_per_analysis * 2),
            samples_per_analysis,
            player_name,
            last_emission_time: 0,
            // Emit at most every 50 ms, so a continuously speaking player cannot
            // flood the webview with activity events.
            emission_cooldown_ms: 50,
        }
    }

    pub fn analyze_samples(&mut self, samples: &[f32]) {
        // Accumulate samples for analysis
        self.sample_accumulator.extend_from_slice(samples);

        // Process chunks when we have enough samples
        while self.sample_accumulator.len() >= self.samples_per_analysis {
            let chunk: Vec<f32> = self
                .sample_accumulator
                .drain(..self.samples_per_analysis)
                .collect();

            self.process_chunk(&chunk);
        }
    }

    fn process_chunk(&mut self, samples: &[f32]) {
        // Calculate RMS (Root Mean Square) for audio level
        let rms = self.calculate_rms(samples);

        // Check if above threshold and cooldown period has passed
        if rms > self.rms_threshold {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            if current_time - self.last_emission_time >= self.emission_cooldown_ms {
                self.emit_activity(rms);
                self.last_emission_time = current_time;
            }
        }
    }

    fn calculate_rms(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    fn emit_activity(&self, rms_level: f32) {
        if let Some(ref tx) = self.activity_tx {
            let update = ActivityUpdate {
                player_name: self.player_name.clone(),
                rms_level,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            };

            // Non-blocking send - if channel is full, we'll just drop this update
            let _ = tx.try_send(update);
        }
    }

    #[allow(dead_code)]
    pub fn set_threshold(&mut self, threshold: f32) {
        self.rms_threshold = threshold;
    }

    #[allow(dead_code)]
    pub fn set_activity_sender(&mut self, tx: flume::Sender<ActivityUpdate>) {
        self.activity_tx = Some(tx);
    }
}
