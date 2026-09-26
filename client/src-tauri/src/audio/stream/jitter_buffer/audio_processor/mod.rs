use log::warn;
use opus2::{Channels, Decoder};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split},
};
use tauri_plugin_curia::curia;

const MAX_OPUS_FRAME_MS: usize = 480;

// Consecutive frames concealed before concealment gives way to silence.
const CONCEALED_FRAMES: usize = 5;

mod error;

pub use error::AudioProcessorError;

pub struct AudioProcessor {
    decoder: Decoder,
    pub current_sample_rate: u32,
    decode_buffer: Vec<f32>,
    output_producer: ringbuf::HeapProd<f32>,
    output_consumer: ringbuf::HeapCons<f32>,

    // Frame management
    pub samples_per_frame: usize,
    pub frame_sample_countdown: usize,
    pub queued_frames: usize,

    // Error handling
    plc_consecutive_count: usize,
    decode_error_count: usize,

    // Samples of a frame decoded but not yet queued, awaiting a play-or-shed decision.
    held_samples: usize,

    // Length of every crossfade and fade, 5 ms at the stream rate.
    fade_samples: usize,
    // The head of the first frame shed since the last queued frame. It continues the audio
    // already queued, so the next queued frame crossfades out of it rather than cutting in.
    shed_head: Vec<f32>,
    // The last queued frame was forced silence, so the next real frame fades in.
    silenced: bool,
}

impl AudioProcessor {
    pub fn new(sample_rate: u32, capacity_frames: usize) -> Result<Self, AudioProcessorError> {
        let decoder =
            Decoder::new(sample_rate, Channels::Mono).map_err(AudioProcessorError::DecoderError)?;

        let samples_per_frame = Self::frames_for_rate(sample_rate);
        let max_samples = Self::max_samples_for_rate(sample_rate);

        let mut decode_buffer = Vec::with_capacity(max_samples);
        decode_buffer.resize(max_samples, 0.0);

        let ring_buf = HeapRb::<f32>::new(capacity_frames * samples_per_frame);
        let (output_producer, output_consumer) = ring_buf.split();

        Ok(Self {
            decoder,
            current_sample_rate: sample_rate,
            decode_buffer,
            output_producer,
            output_consumer,
            samples_per_frame,
            frame_sample_countdown: 0,
            queued_frames: 0,
            plc_consecutive_count: 0,
            decode_error_count: 0,
            held_samples: 0,
            fade_samples: (sample_rate as usize) / 200,
            shed_head: Vec::new(),
            silenced: false,
        })
    }

    fn frames_for_rate(rate: u32) -> usize {
        (rate as usize) / 50 // 20ms frames
    }

    fn max_samples_for_rate(rate: u32) -> usize {
        (rate as usize) * MAX_OPUS_FRAME_MS / 1000
    }

    /// Decodes a frame and holds its samples without queuing them, returning their RMS.
    ///
    /// The decoder advances past the frame either way, so whether the held samples are then
    /// played or discarded, the next frame decodes without a discontinuity.
    pub fn decode_held(&mut self, opus_data: &[u8]) -> Result<f32, AudioProcessorError> {
        let samples_written =
            match self
                .decoder
                .decode_float(opus_data, &mut self.decode_buffer, false)
            {
                Ok(samples) => samples,
                Err(e) => {
                    self.decode_error_count += 1;

                    if self.decode_error_count >= 10 {
                        curia::warn!("multiple consecutive decode errors, resetting decoder", {
                            defect: crate::logging::Defect::DecoderResetLoop,
                            io: "output",
                        });
                        self.reset_decoder()?;
                    }

                    return Err(AudioProcessorError::DecoderError(e));
                }
            };

        // The decoder resets only on consecutive errors; a good frame ends the run.
        self.decode_error_count = 0;
        self.held_samples = samples_written;
        Ok(Self::rms(&self.decode_buffer[..samples_written]))
    }

    /// Queues the held frame for playback, returning the frames written.
    pub fn commit_held(&mut self) -> usize {
        let mut held: Vec<f32> = self.decode_buffer[..self.held_samples].to_vec();
        self.held_samples = 0;
        self.smooth_join(&mut held);
        self.write_samples_to_ring(&held)
    }

    /// Drops the held frame without playing it.
    pub fn discard_held(&mut self) {
        if self.shed_head.is_empty() {
            let len = self.held_samples.min(self.fade_samples);
            self.shed_head = self.decode_buffer[..len].to_vec();
        }
        self.held_samples = 0;
    }

    /// Removes the step where a frame joins audio it does not continue: crossfades out of a shed
    /// frame's head, or fades in after forced silence.
    fn smooth_join(&mut self, frame: &mut [f32]) {
        if !self.shed_head.is_empty() {
            let len = self.shed_head.len().min(frame.len());
            for (i, (sample, shed)) in frame.iter_mut().zip(&self.shed_head).enumerate() {
                let w = i as f32 / len as f32;
                *sample = shed * (1.0 - w) + *sample * w;
            }
            self.shed_head.clear();
        } else if self.silenced {
            let len = self.fade_samples.min(frame.len());
            for (i, sample) in frame.iter_mut().take(len).enumerate() {
                *sample *= i as f32 / len as f32;
            }
        }
        self.silenced = false;
    }

    /// Decode opus data and write samples to ring buffer
    pub fn decode_opus(&mut self, opus_data: &[u8]) -> Result<usize, AudioProcessorError> {
        self.decode_held(opus_data)?;
        Ok(self.commit_held())
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    /// Write samples to ring buffer in frame-sized chunks
    fn write_samples_to_ring(&mut self, samples: &[f32]) -> usize {
        let mut frames_written = 0;

        for chunk in samples.chunks(self.samples_per_frame) {
            // Write chunk to ring buffer
            for &sample in chunk {
                if self.output_producer.try_push(sample).is_err() {
                    warn!("Ring buffer overflow during decode");
                    return frames_written;
                }
            }

            // Pad frame if needed
            if chunk.len() < self.samples_per_frame {
                for _ in chunk.len()..self.samples_per_frame {
                    if self.output_producer.try_push(0.0).is_err() {
                        return frames_written;
                    }
                }
            }

            frames_written += 1;
        }

        self.queued_frames = self.queued_frames.saturating_add(frames_written);
        frames_written
    }

    /// Generate PLC (Packet Loss Concealment) directly to ring buffer
    pub fn generate_plc(&mut self) -> Result<(), AudioProcessorError> {
        self.plc_consecutive_count += 1;

        let concealing = self.plc_consecutive_count <= CONCEALED_FRAMES;
        let mut frame = vec![0.0f32; self.samples_per_frame];

        if concealing {
            // One frame of output requests one frame of concealment. With an empty packet libopus
            // conceals for as long as the output buffer is, and the whole buffer is 480 ms.
            let plc_samples = match self.decoder.decode_float(
                &[],
                &mut self.decode_buffer[..self.samples_per_frame],
                true,
            ) {
                Ok(samples) => samples,
                Err(_) => self.samples_per_frame,
            };
            frame[..plc_samples].copy_from_slice(&self.decode_buffer[..plc_samples]);

            // The last concealed frame fades out, so the silence after it does not cut in.
            if self.plc_consecutive_count == CONCEALED_FRAMES {
                let len = self.fade_samples.min(frame.len());
                let start = frame.len() - len;
                for (i, sample) in frame[start..].iter_mut().enumerate() {
                    *sample *= 1.0 - (i + 1) as f32 / len as f32;
                }
            }
            self.smooth_join(&mut frame);
        } else {
            // Silence after too many consecutive PLC attempts
            self.shed_head.clear();
            self.silenced = true;
        }

        for sample in frame {
            if self.output_producer.try_push(sample).is_err() {
                return Err(AudioProcessorError::RingBufferFull);
            }
        }

        self.queued_frames = self.queued_frames.saturating_add(1);
        Ok(())
    }

    /// Queues one frame of silence for playback.
    pub fn push_silence_frame(&mut self) {
        let silence = vec![0.0f32; self.samples_per_frame];
        self.write_samples_to_ring(&silence);
        self.shed_head.clear();
        self.silenced = true;
    }

    /// Get next audio sample from ring buffer
    pub fn next_sample(&mut self) -> Option<f32> {
        if let Some(sample) = self.output_consumer.try_pop() {
            // Update frame countdown
            if self.frame_sample_countdown == 0 {
                self.frame_sample_countdown = self.samples_per_frame;
            }
            self.frame_sample_countdown = self.frame_sample_countdown.saturating_sub(1);

            // Complete frame consumed
            if self.frame_sample_countdown == 0 {
                if self.queued_frames > 0 {
                    self.queued_frames -= 1;
                }
            }

            Some(sample)
        } else {
            None
        }
    }

    /// Reset decoder on consecutive errors
    fn reset_decoder(&mut self) -> Result<(), AudioProcessorError> {
        self.decoder = Decoder::new(self.current_sample_rate, Channels::Mono)
            .map_err(AudioProcessorError::DecoderError)?;
        self.decode_error_count = 0;
        Ok(())
    }

    /// Reset PLC counter on successful decode
    pub fn reset_plc_counter(&mut self) {
        self.plc_consecutive_count = 0;
    }

    /// Check if ring buffer has samples available
    pub fn has_samples(&self) -> bool {
        self.queued_frames > 0
    }
}
