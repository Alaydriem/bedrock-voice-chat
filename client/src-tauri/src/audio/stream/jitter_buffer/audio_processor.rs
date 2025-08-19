use opus::{Channels, Decoder};
use ringbuf::{HeapRb, traits::{Split, Producer, Consumer}};
use log::warn;

const MAX_OPUS_FRAME_MS: usize = 480; // worst-case single decode span

#[derive(Debug)]
pub enum AudioProcessorError {
    DecoderError(opus::Error),
    RingBufferFull,
}

impl std::fmt::Display for AudioProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioProcessorError::DecoderError(e) => write!(f, "Opus decoder error: {:?}", e),
            AudioProcessorError::RingBufferFull => write!(f, "Ring buffer is full"),
        }
    }
}

impl std::error::Error for AudioProcessorError {}

/// Core audio processing state - focused solely on decoding and output
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
}

impl AudioProcessor {
    pub fn new(sample_rate: u32, capacity_frames: usize) -> Result<Self, AudioProcessorError> {
        let decoder = Decoder::new(sample_rate, Channels::Mono)
            .map_err(AudioProcessorError::DecoderError)?;
        
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
        })
    }
    
    fn frames_for_rate(rate: u32) -> usize {
        (rate as usize) / 50 // 20ms frames
    }
    
    fn max_samples_for_rate(rate: u32) -> usize {
        (rate as usize) * MAX_OPUS_FRAME_MS / 1000
    }
    
    /// Decode opus data and write samples to ring buffer
    pub fn decode_opus(&mut self, opus_data: &[u8]) -> Result<usize, AudioProcessorError> {
        // Decode to buffer first
        let samples_written = match self.decoder.decode_float(opus_data, &mut self.decode_buffer, false) {
            Ok(samples) => {
                self.decode_error_count = 0;
                samples
            }
            Err(e) => {
                self.decode_error_count += 1;
                
                if self.decode_error_count >= 10 {
                    warn!("Multiple consecutive decode errors, resetting decoder");
                    self.reset_decoder()?;
                }
                
                return Err(AudioProcessorError::DecoderError(e));
            }
        };
        
        // Copy the decoded samples to avoid borrowing conflicts
        let decoded_samples: Vec<f32> = self.decode_buffer[..samples_written].to_vec();
        let frames_written = self.write_samples_to_ring(&decoded_samples);
        Ok(frames_written)
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
        
        let plc_samples = if self.plc_consecutive_count <= 5 {
            // Use decoder's built-in PLC for first few attempts
            match self.decoder.decode_float(&[], &mut self.decode_buffer, true) {
                Ok(samples) => samples,
                Err(_) => self.samples_per_frame,
            }
        } else {
            // Generate silence after too many consecutive PLC attempts
            self.samples_per_frame
        };
        
        // Write PLC samples to ring
        for i in 0..self.samples_per_frame {
            let sample = if i < plc_samples && self.plc_consecutive_count <= 5 {
                self.decode_buffer[i]
            } else {
                0.0 // Silence
            };
            
            if self.output_producer.try_push(sample).is_err() {
                return Err(AudioProcessorError::RingBufferFull);
            }
        }
        
        self.queued_frames = self.queued_frames.saturating_add(1);
        Ok(())
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
