#[cfg(feature = "e2e")]
mod bridge;
mod config;

#[cfg(feature = "e2e")]
pub use bridge::BridgeInputSource;
pub(crate) use config::CaptureConfig;

use crate::audio::stream::StreamRecoveryEvent;
use crate::audio::types::{AudioDevice, AudioDeviceCpal, AudioDeviceType};
use anyhow::anyhow;
use log::{error, warn};
use rodio::DeviceTrait;
use rodio::cpal::traits::StreamTrait as CpalStreamTrait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;

// Driver returned by a source after it begins producing frames: the tokio handle
// the sender task is paired against, plus the oneshot the listener holds to stop
// a live cpal stream (None for sources that stop on their own feed closing).
pub(crate) struct SourceDriver {
    pub handle: JoinHandle<()>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

// Pull/push source of raw f32 frames fed through the real input processing path.
// Enum dispatch, no trait objects: both variants resolve a uniform CaptureConfig
// and drive an identical push sink (`FnMut(&[f32])`), so the listener has one
// body and the production cpal path keeps its callback-driven, queue-free timing.
pub(crate) enum AudioInputSource {
    Cpal,
    #[cfg(feature = "e2e")]
    Fake(BridgeInputSource),
}

impl AudioInputSource {
    // Resolves capture parameters for the active variant. The cpal path reads the
    // live device (validating the stored config against the hardware); the fake
    // path reports the bridge's own rate/channels with an f32 sample format.
    pub(crate) fn resolve_config(
        &self,
        device: &Option<AudioDevice>,
    ) -> Result<CaptureConfig, anyhow::Error> {
        match self {
            Self::Cpal => {
                let device = device.clone().ok_or_else(|| {
                    anyhow!("InputStream is not initialized with a device! Unable to start stream")
                })?;
                let stored_config = device.get_stream_config()?;

                let config = match crate::audio::device::refresh_device_config(&device) {
                    Some(fresh_configs) if !fresh_configs.is_empty() => {
                        let fresh_config: rodio::cpal::SupportedStreamConfig =
                            fresh_configs[0].clone().into();
                        if fresh_config.sample_rate() != stored_config.sample_rate() {
                            warn!(
                                "Device {} sample rate changed: stored {}Hz, actual {}Hz. Using actual.",
                                device.display_name,
                                stored_config.sample_rate(),
                                fresh_config.sample_rate()
                            );
                        }
                        fresh_config
                    }
                    _ => {
                        warn!(
                            "Could not refresh device config for {}, using stored config",
                            device.display_name
                        );
                        stored_config
                    }
                };

                // Mobile audio backends (CoreAudio on iOS, AAudio on Android)
                // should use the default buffer size, otherwise the input stream
                // may fail to initialize or produce no audio
                #[cfg(any(target_os = "ios", target_os = "android"))]
                let buffer_size = rodio::cpal::BufferSize::Default;

                #[cfg(not(any(target_os = "ios", target_os = "android")))]
                let buffer_size = rodio::cpal::BufferSize::Fixed(crate::audio::types::BUFFER_SIZE);

                Ok(CaptureConfig {
                    sample_rate: config.sample_rate(),
                    channels: config.channels(),
                    sample_format: config.sample_format(),
                    buffer_size,
                })
            }
            #[cfg(feature = "e2e")]
            Self::Fake(src) => Ok(CaptureConfig {
                sample_rate: src.sample_rate(),
                channels: src.channels(),
                sample_format: rodio::cpal::SampleFormat::F32,
                buffer_size: rodio::cpal::BufferSize::Default,
            }),
        }
    }

    // Begins producing frames, pushing each into `process`. The cpal variant
    // builds a live input stream whose callback invokes `process`; the fake
    // variant runs the bridge feed loop invoking the same closure. Both keep the
    // real work on an OS thread and return a tokio handle for the sender to pair
    // against, plus (cpal only) the oneshot used to stop the stream.
    pub(crate) fn drive<F>(
        self,
        config: CaptureConfig,
        device: Option<AudioDevice>,
        process: F,
        shutdown: Arc<AtomicBool>,
        recovery_tx: crate::audio::stream::RecoverySender,
    ) -> Result<SourceDriver, anyhow::Error>
    where
        F: FnMut(&[f32]) + Send + 'static,
    {
        match self {
            Self::Cpal => Self::drive_cpal(config, device, process, shutdown, recovery_tx),
            #[cfg(feature = "e2e")]
            Self::Fake(src) => Ok(Self::drive_fake(src, process, shutdown)),
        }
    }

    fn drive_cpal<F>(
        config: CaptureConfig,
        device: Option<AudioDevice>,
        process: F,
        shutdown: Arc<AtomicBool>,
        recovery_tx: crate::audio::stream::RecoverySender,
    ) -> Result<SourceDriver, anyhow::Error>
    where
        F: FnMut(&[f32]) + Send + 'static,
    {
        let device = device.ok_or_else(|| {
            anyhow!("InputStream is not initialized with a device! Unable to start stream")
        })?;

        let cpal_device = device.clone().to_cpal_device().ok_or_else(|| {
            error!(
                "CPAL device not found for {} '{}'. Device may have been disconnected or its ID changed. {:?}",
                device.io.store_key(),
                device.display_name,
                device
            );
            anyhow!(
                "Couldn't retrieve native cpal device for {} {}.",
                device.io.store_key(),
                device.display_name
            )
        })?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let sample_format = config.sample_format;
        let device_config = rodio::cpal::StreamConfig {
            channels: config.channels,
            sample_rate: config.sample_rate,
            buffer_size: config.buffer_size,
        };

        std::thread::Builder::new()
            .name("audio-input".into())
            .spawn(move || {
                let recovery_tx_for_error = recovery_tx.clone();
                let shutdown_for_error = shutdown.clone();

                log::info!(
                    "Input Stream Config: {:?} {:?}",
                    device_config.channels,
                    device_config.sample_rate
                );

                let error_fn = move |error: rodio::cpal::StreamError| {
                    error!("Audio stream error (device may have disconnected): {}", error);
                    shutdown_for_error.store(true, Ordering::Relaxed);

                    let _ = recovery_tx_for_error.send(StreamRecoveryEvent::DeviceError {
                        device_type: AudioDeviceType::InputDevice,
                        error: error.to_string(),
                    });
                };

                // Wrap process in Arc<Mutex<>> so it can be shared across the
                // fixed/default buffer-size fallback attempts.
                let process = std::sync::Arc::new(std::sync::Mutex::new(process));

                let build_stream = |cfg: &rodio::cpal::StreamConfig,
                                    process: std::sync::Arc<std::sync::Mutex<dyn FnMut(&[f32]) + Send>>,
                                    err_fn: Box<dyn FnMut(rodio::cpal::StreamError) + Send>|
                    -> Result<rodio::cpal::Stream, rodio::cpal::BuildStreamError> {
                    match sample_format {
                        rodio::cpal::SampleFormat::F32 => {
                            let process = process.clone();
                            cpal_device.build_input_stream(
                                cfg,
                                move |data: &[f32], _: &rodio::cpal::InputCallbackInfo| {
                                    if let Ok(mut pf) = process.lock() {
                                        pf(data);
                                    }
                                },
                                err_fn,
                                None,
                            )
                        }
                        rodio::cpal::SampleFormat::I32 => {
                            let process = process.clone();
                            cpal_device.build_input_stream(
                                cfg,
                                move |data: &[i32], _: &rodio::cpal::InputCallbackInfo| {
                                    const SCALE: f32 = 2147483648.0;
                                    let f32_data: Vec<f32> = data
                                        .iter()
                                        .map(|&sample| sample as f32 / SCALE)
                                        .collect();
                                    if let Ok(mut pf) = process.lock() {
                                        pf(&f32_data);
                                    }
                                },
                                err_fn,
                                None,
                            )
                        }
                        rodio::cpal::SampleFormat::I16 => {
                            let process = process.clone();
                            cpal_device.build_input_stream(
                                cfg,
                                move |data: &[i16], _: &rodio::cpal::InputCallbackInfo| {
                                    const SCALE: f32 = 32768.0;
                                    let f32_data: Vec<f32> = data
                                        .iter()
                                        .map(|&sample| sample as f32 / SCALE)
                                        .collect();
                                    if let Ok(mut pf) = process.lock() {
                                        pf(&f32_data);
                                    }
                                },
                                err_fn,
                                None,
                            )
                        }
                        _ => Err(rodio::cpal::BuildStreamError::StreamConfigNotSupported),
                    }
                };

                let stream = build_stream(&device_config, process.clone(), Box::new(error_fn));

                // If StreamConfigNotSupported and we used Fixed buffer, retry with Default
                let stream = match stream {
                    Err(rodio::cpal::BuildStreamError::StreamConfigNotSupported)
                        if device_config.buffer_size != rodio::cpal::BufferSize::Default =>
                    {
                        warn!(
                            "Fixed buffer size not supported for input {}, falling back to default buffer size",
                            device.display_name
                        );
                        let fallback_config = rodio::cpal::StreamConfig {
                            buffer_size: rodio::cpal::BufferSize::Default,
                            ..device_config
                        };
                        let shutdown_retry = shutdown.clone();
                        let recovery_tx_retry = recovery_tx.clone();
                        let fallback_error_fn = move |error: rodio::cpal::StreamError| {
                            error!("Audio input stream error (device may have disconnected): {}", error);
                            shutdown_retry.store(true, Ordering::Relaxed);
                            let _ = recovery_tx_retry.send(StreamRecoveryEvent::DeviceError {
                                device_type: AudioDeviceType::InputDevice,
                                error: error.to_string(),
                            });
                        };
                        build_stream(&fallback_config, process, Box::new(fallback_error_fn))
                    }
                    other => other,
                };

                match stream {
                    Ok(stream) => {
                        if let Err(e) = stream.play() {
                            error!("Failed to start input audio stream: {:?}", e);
                            shutdown.store(true, Ordering::Relaxed);
                            let _ = recovery_tx.send(StreamRecoveryEvent::DeviceError {
                                device_type: AudioDeviceType::InputDevice,
                                error: format!("Failed to start input stream: {:?}", e),
                            });
                            return;
                        }

                        let _ = shutdown_rx.blocking_recv();

                        if let Err(e) = stream.pause() {
                            warn!("Failed to pause stream (may already be stopped): {:?}", e);
                        }
                        drop(stream);
                    }
                    Err(e) => {
                        error!("{:?}", e);
                    }
                };
            })?;

        // Real work runs on the OS thread; the sender task pairs against this
        // tokio handle.
        let handle = tokio::spawn(async {});
        Ok(SourceDriver {
            handle,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    #[cfg(feature = "e2e")]
    fn drive_fake<F>(
        mut src: BridgeInputSource,
        mut process: F,
        shutdown: Arc<AtomicBool>,
    ) -> SourceDriver
    where
        F: FnMut(&[f32]) + Send + 'static,
    {
        std::thread::Builder::new()
            .name("audio-input-fake".into())
            .spawn(move || {
                src.drive(|frame| {
                    if shutdown.load(Ordering::Relaxed) {
                        return;
                    }
                    process(frame);
                });
            })
            .expect("failed to spawn fake audio-input thread");

        let handle = tokio::spawn(async {});
        SourceDriver {
            handle,
            shutdown_tx: None,
        }
    }
}
