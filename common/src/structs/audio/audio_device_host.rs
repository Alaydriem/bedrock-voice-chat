use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[cfg(feature = "audio")]
use rodio::cpal::HostId;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AudioDeviceHost {
    #[cfg(target_os = "windows")]
    Asio,
    #[cfg(target_os = "windows")]
    Wasapi,
    #[cfg(target_os = "android")]
    AAudio,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    CoreAudio,
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    Alsa,
}

#[cfg(feature = "audio")]
impl TryFrom<rodio::cpal::HostId> for AudioDeviceHost {
    type Error = ();

    fn try_from(value: rodio::cpal::HostId) -> Result<Self, Self::Error> {
        #[allow(unreachable_patterns)]
        match value {
            #[cfg(target_os = "windows")]
            HostId::Asio => Ok(AudioDeviceHost::Asio),
            #[cfg(target_os = "windows")]
            HostId::Wasapi => Ok(AudioDeviceHost::Wasapi),
            #[cfg(target_os = "android")]
            HostId::AAudio => Ok(AudioDeviceHost::AAudio),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            HostId::CoreAudio => Ok(AudioDeviceHost::CoreAudio),
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd"
            ))]
            HostId::Alsa => Ok(AudioDeviceHost::Alsa),
            _ => Err(()),
        }
    }
}

#[cfg(feature = "audio")]
impl Into<rodio::cpal::HostId> for AudioDeviceHost {
    fn into(self) -> rodio::cpal::HostId {
        let host: rodio::cpal::HostId;
        #[cfg(target_os = "windows")]
        {
            host = match self {
                AudioDeviceHost::Asio => HostId::Asio,
                AudioDeviceHost::Wasapi => HostId::Wasapi,
            };
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ))]
        {
            host = match self {
                AudioDeviceHost::Alsa => HostId::Alsa,
            };
        }
        #[cfg(target_os = "android")]
        {
            host = match self {
                AudioDeviceHost::AAudio => HostId::AAudio,
            };
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            host = match self {
                AudioDeviceHost::CoreAudio => HostId::CoreAudio,
            };
        }

        host
    }
}
