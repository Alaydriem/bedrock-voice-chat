mod chime;

// Public because the waveform is the one part of a speaker test that can be asserted
// without a speaker. `SpeakerTest` itself stays crate-internal: opening a device and
// sleeping for the duration is not something a test can observe.
pub use chime::Chime;

use crate::audio::types::{AudioDevice, AudioDeviceCpal};
use anyhow::anyhow;
use log::error;
use rodio::DeviceSinkBuilder;
use rodio::buffer::SamplesBuffer;

/// Plays a chime through a chosen output device.
///
/// Standalone by design: it opens its own short-lived stream rather than borrowing the
/// session's. The session output stream needs a current player, a packet consumer and the
/// spatial mixer, none of which exist during setup — and none of which a speaker test has
/// any business depending on.
///
/// It does go through the device the user picked, not the system default. A test that played
/// out of whatever the platform felt like would confirm nothing about the device BVC is
/// about to use, which is the only question being asked.
pub(crate) struct SpeakerTest;

impl SpeakerTest {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Play the chime and return once it has finished.
    ///
    /// Blocking, and run on a blocking thread by the caller: the rodio stream is not `Send`,
    /// and dropping it stops playback, so it has to stay on one thread for the duration
    /// rather than be held across an await.
    pub(crate) fn play(&self, device: AudioDevice) -> Result<(), anyhow::Error> {
        let config = device.get_stream_config()?;
        let sample_rate = config.sample_rate();
        let channels = config.channels();

        let cpal_device = device.clone().to_cpal_device().ok_or_else(|| {
            anyhow!(
                "Couldn't retrieve native cpal device for {}",
                device.display_name
            )
        })?;

        let stream_config: rodio::cpal::StreamConfig = config.into();
        let mut stream = DeviceSinkBuilder::from_device(cpal_device)
            .map_err(|e| {
                error!("Speaker test could not create DeviceSinkBuilder: {:?}", e);
                anyhow!(e)
            })?
            .with_config(&stream_config)
            .open_sink_or_fallback()
            .map_err(|e| {
                error!("Speaker test could not open the output device: {:?}", e);
                anyhow!(e)
            })?;
        stream.log_on_drop(false);

        let buffer_channels = std::num::NonZeroU16::new(channels)
            .ok_or_else(|| anyhow!("Output device {} reports no channels", device.display_name))?;
        let buffer_rate = std::num::NonZeroU32::new(sample_rate).ok_or_else(|| {
            anyhow!(
                "Output device {} reports no sample rate",
                device.display_name
            )
        })?;

        // Generated at the device's own rate, so nothing resamples on the way out.
        let samples = Chime::samples(sample_rate, channels);
        stream
            .mixer()
            .add(SamplesBuffer::new(buffer_channels, buffer_rate, samples));

        // Held for the length of the chime plus a little, because dropping the stream cuts
        // playback mid-note. The tail also covers the device's own buffering, which would
        // otherwise clip the decay on a large buffer size.
        let hold = std::time::Duration::from_secs_f32(Chime::DURATION_SECONDS + 0.25);
        std::thread::sleep(hold);

        Ok(())
    }
}
