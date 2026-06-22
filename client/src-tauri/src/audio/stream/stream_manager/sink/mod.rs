#[cfg(feature = "e2e")]
mod capturing;
mod mix_target;

#[cfg(feature = "e2e")]
pub use capturing::CapturingSink;
pub(crate) use mix_target::MixTarget;

use crate::audio::types::{AudioDevice, AudioDeviceCpal};
use anyhow::anyhow;
use log::error;
use rodio::DeviceSinkBuilder;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

// Post-mix consumer for output samples. Enum dispatch, no trait objects: each
// variant yields a uniform MixTarget so playback never branches on the sink kind.
pub(crate) enum AudioOutputSink {
    Rodio,
    #[cfg(feature = "e2e")]
    Fake(CapturingSink),
}

impl AudioOutputSink {
    // Opens the mix target for the active variant. The cpal path acquires a live
    // device sink and exposes its mixer; the fake path builds an in-memory mixer
    // and spawns a real-time drain that captures the post-mix PCM.
    pub(crate) fn open(
        self,
        device: Option<AudioDevice>,
        config: Option<rodio::cpal::SupportedStreamConfig>,
        #[cfg_attr(not(feature = "e2e"), allow(unused_variables))] shutdown: Arc<AtomicBool>,
    ) -> Result<MixTarget, anyhow::Error> {
        match self {
            Self::Rodio => Self::open_rodio(device, config),
            #[cfg(feature = "e2e")]
            Self::Fake(cap) => Self::open_fake(cap, shutdown),
        }
    }

    fn open_rodio(
        device: Option<AudioDevice>,
        config: Option<rodio::cpal::SupportedStreamConfig>,
    ) -> Result<MixTarget, anyhow::Error> {
        let device = device.ok_or_else(|| {
            anyhow!("Output Stream is not initialized with a device! Unable to start stream")
        })?;
        let config =
            config.ok_or_else(|| anyhow!("Output device is missing a usable stream config"))?;

        let cpal_device = device.clone().to_cpal_device().ok_or_else(|| {
            error!(
                "CPAL output device is not defined. This shouldn't happen! Restart BVC? {:?}",
                device.clone()
            );
            anyhow!(
                "Couldn't retrieve native cpal device for {} {}.",
                device.io.store_key(),
                device.display_name
            )
        })?;

        log::info!("started receiving audio stream");
        let builder = DeviceSinkBuilder::from_device(cpal_device).map_err(|e| {
            error!("Could not create DeviceSinkBuilder: {:?}", e);
            anyhow!(e)
        })?;

        let stream_config: rodio::cpal::StreamConfig = config.into();
        let builder = builder.with_config(&stream_config);
        let mut stream = builder.open_sink_or_fallback().map_err(|e| {
            error!(
                "Could not acquire MixerDeviceSink. Try restarting the stream? {:?}",
                e
            );
            anyhow!(e)
        })?;

        stream.log_on_drop(false);
        let mixer = Arc::new(stream.mixer().clone());

        Ok(MixTarget {
            mixer,
            playback_stream: Some(stream),
        })
    }

    #[cfg(feature = "e2e")]
    fn open_fake(
        cap: CapturingSink,
        shutdown: Arc<AtomicBool>,
    ) -> Result<MixTarget, anyhow::Error> {
        use std::sync::atomic::Ordering;

        let channels = std::num::NonZeroU16::new(cap.channels())
            .ok_or_else(|| anyhow!("fake sink channels must be non-zero"))?;
        let sample_rate = std::num::NonZeroU32::new(cap.sample_rate())
            .ok_or_else(|| anyhow!("fake sink sample_rate must be non-zero"))?;

        let (mix, mut source) = rodio::mixer::mixer(channels, sample_rate);

        // MixerSource::next() yields None whenever it has no active sources, so
        // keep an infinite silent source registered to keep the drain alive.
        mix.add(rodio::source::Zero::new(channels, sample_rate));

        // ~20ms blocks: sample_rate/50 frames * channels samples per pull
        let frames = (cap.sample_rate() / 50) as usize;
        let block_len = frames * cap.channels() as usize;

        std::thread::Builder::new()
            .name("audio-output-fake".into())
            .spawn(move || {
                let mut clock = super::frame_clock::FrameClock::new(20.0);
                while !shutdown.load(Ordering::Relaxed) {
                    let mut block = Vec::with_capacity(block_len);
                    for _ in 0..block_len {
                        match source.next() {
                            Some(s) => block.push(s),
                            None => break,
                        }
                    }
                    cap.submit(block);
                    clock.wait_next();
                }
            })?;

        Ok(MixTarget {
            mixer: Arc::new(mix),
            playback_stream: None,
        })
    }
}
