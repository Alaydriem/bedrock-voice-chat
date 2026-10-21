#[cfg(feature = "e2e")]
mod bridge;
mod config;
mod driver;

#[cfg(feature = "e2e")]
pub use bridge::BridgeInputSource;
pub(crate) use config::CaptureConfig;
pub(crate) use driver::SourceDriver;

use crate::audio::stream::StreamRecoveryEvent;
use crate::audio::{AudioDevice, AudioDeviceCpal, AudioDeviceType};
use anyhow::anyhow;
use rodio::DeviceTrait;
use rodio::cpal::traits::StreamTrait as CpalStreamTrait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri_plugin_curia::curia;

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
    // WASAPI surfaces rejected stream parameters as a BackendSpecific error
    // (e.g. E_INVALIDARG 0x80070057) rather than StreamConfigNotSupported;
    // both mean the requested config was refused, not that the device is gone.
    fn is_config_rejection(error: &rodio::cpal::BuildStreamError) -> bool {
        matches!(
            error,
            rodio::cpal::BuildStreamError::StreamConfigNotSupported
                | rodio::cpal::BuildStreamError::BackendSpecific { .. }
        )
    }

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

                let config =
                    match crate::audio::device::AudioDeviceEnumerator::refresh_device_config(
                        &device,
                    ) {
                        Some(fresh_configs) if !fresh_configs.is_empty() => {
                            let fresh_config: rodio::cpal::SupportedStreamConfig =
                                fresh_configs[0].clone().into();
                            if fresh_config.sample_rate() != stored_config.sample_rate() {
                                curia::warn!("input device sample rate changed; using the actual rate", {
                                    io: "input",
                                    device_name: device.display_name.clone(),
                                    stored_rate: stored_config.sample_rate(),
                                    actual_rate: fresh_config.sample_rate(),
                                });
                            }
                            fresh_config
                        }
                        // The stored snapshot is the last resort: a live default
                        // config reflects what the endpoint accepts right now,
                        // while the snapshot can predate a Windows format change.
                        _ => match device
                            .clone()
                            .to_cpal_device()
                            .and_then(|d| d.default_input_config().ok())
                        {
                            Some(live_config) => {
                                curia::warn!("could not refresh the input device config; using the live default config", {
                                    io: "input",
                                    device_name: device.display_name.clone(),
                                });
                                live_config
                            }
                            None => {
                                curia::warn!("could not refresh the input device config; using the stored config", {
                                    io: "input",
                                    device_name: device.display_name.clone(),
                                });
                                stored_config
                            }
                        },
                    };

                // Mobile audio backends (CoreAudio on iOS, AAudio on Android)
                // should use the default buffer size, otherwise the input stream
                // may fail to initialize or produce no audio
                #[cfg(any(target_os = "ios", target_os = "android"))]
                let buffer_size = rodio::cpal::BufferSize::Default;

                #[cfg(not(any(target_os = "ios", target_os = "android")))]
                let buffer_size = rodio::cpal::BufferSize::Fixed(crate::audio::BUFFER_SIZE);

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

    // Begins producing frames, pushing each into `process`. The cpal variant builds a live input
    // stream whose callback invokes `process` and hands the stream back to be held; the fake
    // variant runs the bridge feed loop on its own thread, invoking the same closure.
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
            curia::error!("CPAL device not found; it may have been disconnected or its ID changed", {
                io: "input",
                store_key: device.io.store_key().to_string(),
                device_name: device.display_name.clone(),
                device: format!("{device:?}"),
            });
            anyhow!(
                "Couldn't retrieve native cpal device for {} {}.",
                device.io.store_key(),
                device.display_name
            )
        })?;

        let sample_format = config.sample_format;
        let device_config = rodio::cpal::StreamConfig {
            channels: config.channels,
            sample_rate: config.sample_rate,
            buffer_size: config.buffer_size,
        };

        let recovery_tx_for_error = recovery_tx.clone();
        let shutdown_for_error = shutdown.clone();

        curia::info!("input stream config", {
            io: "input",
            channels: format!("{:?}", device_config.channels),
            sample_rate: format!("{:?}", device_config.sample_rate),
        });

        let error_fn = move |error: rodio::cpal::StreamError| {
            curia::error!("audio stream error; the device may have disconnected", {
                io: "input",
                error: error.to_string(),
            });
            shutdown_for_error.store(true, Ordering::Relaxed);

            let _ = recovery_tx_for_error.send(StreamRecoveryEvent::DeviceError {
                device_type: AudioDeviceType::InputDevice,
                error: error.to_string(),
            });
        };

        // Wrap process in Arc<Mutex<>> so it can be shared across the
        // fixed/default buffer-size fallback attempts.
        let process = std::sync::Arc::new(std::sync::Mutex::new(process));

        // Poisoning is recovered from rather than propagated. A `lock()` that returns
        // `Err` once returns it for the rest of the process, so a callback that skipped
        // the frame on `Err` went silent permanently after any single panic in the
        // processing core — with cpal still reporting a healthy stream, no error
        // callback, and nothing anywhere to say the microphone had stopped.
        //
        // The core's own state may be inconsistent after such a panic; one frame of
        // damaged audio is a far smaller fault than a microphone that never works again.

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
                            let mut pf = process.lock().unwrap_or_else(|e| e.into_inner());
                            pf(data);
                        },
                        err_fn,
                        None,
                    )
                }
                // Both formats are already normalised to [-1.0, 1.0], so the width is the
                // only difference and the cast is the whole conversion.
                rodio::cpal::SampleFormat::F64 => {
                    let process = process.clone();
                    cpal_device.build_input_stream(
                        cfg,
                        move |data: &[f64], _: &rodio::cpal::InputCallbackInfo| {
                            let f32_data: Vec<f32> =
                                data.iter().map(|&sample| sample as f32).collect();
                            let mut pf = process.lock().unwrap_or_else(|e| e.into_inner());
                            pf(&f32_data);
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
                            let f32_data: Vec<f32> =
                                data.iter().map(|&sample| sample as f32 / SCALE).collect();
                            let mut pf = process.lock().unwrap_or_else(|e| e.into_inner());
                            pf(&f32_data);
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
                            let f32_data: Vec<f32> =
                                data.iter().map(|&sample| sample as f32 / SCALE).collect();
                            let mut pf = process.lock().unwrap_or_else(|e| e.into_inner());
                            pf(&f32_data);
                        },
                        err_fn,
                        None,
                    )
                }
                _ => Err(rodio::cpal::BuildStreamError::StreamConfigNotSupported),
            }
        };

        let stream = build_stream(&device_config, process.clone(), Box::new(error_fn));

        // If the config was rejected and we used a Fixed buffer, retry with Default
        let stream = match stream {
            Err(e)
                if Self::is_config_rejection(&e)
                    && device_config.buffer_size != rodio::cpal::BufferSize::Default =>
            {
                curia::warn!("input stream config rejected; falling back to the default buffer size", {
                    io: "input",
                    device_name: device.display_name.clone(),
                    error: format!("{e:?}"),
                });
                let fallback_config = rodio::cpal::StreamConfig {
                    buffer_size: rodio::cpal::BufferSize::Default,
                    ..device_config
                };
                let shutdown_retry = shutdown.clone();
                let recovery_tx_retry = recovery_tx.clone();
                let fallback_error_fn = move |error: rodio::cpal::StreamError| {
                    curia::error!("audio input stream error; the device may have disconnected", {
                        io: "input",
                        error: error.to_string(),
                    });
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
                    shutdown.store(true, Ordering::Relaxed);
                    return Err(anyhow!("Failed to start input stream: {:?}", e));
                }

                Ok(SourceDriver {
                    stream: Some(stream),
                })
            }
            // A stream that never opened is a failed start, not a start that produced a broken
            // stream. Reporting it as `Ok` set `capture_expected`, which armed the capture
            // watchdog over a device that had never been opened, and the recovery event it also
            // sent had the webview rebuilding it every 500 ms. Neither loop could ever succeed:
            // both buffer sizes were already refused for this endpoint.
            Err(e) => {
                shutdown.store(true, Ordering::Relaxed);
                Err(anyhow!("Failed to build input stream: {:?}", e))
            }
        }
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

        // No cpal stream to hold: the bridge feed ends when its own source closes, and the
        // shutdown flag the closure above reads is what stops it early.
        SourceDriver { stream: None }
    }
}
