use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AnalyticsEvent {
    AppStarted,
    AppExited,
    Reload,
    AudioDeviceChanged,
    JavaIdentityLinked,
    RecordingExported,
    Logout,
    LoginCompleted,
    ServerChanged,
    AnalyticsToggled,
    OnboardingCompleted,
    NoiseGateToggled,
    VoiceModeChanged,
    WebsocketServerToggled,
    ChannelJoined,
    ChannelLeft,
    RecordingStarted,
    RecordingStopped,
}

impl AnalyticsEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::AppStarted => "AppStarted",
            Self::AppExited => "AppExited",
            Self::Reload => "Reload",
            Self::AudioDeviceChanged => "AudioDeviceChanged",
            Self::JavaIdentityLinked => "JavaIdentityLinked",
            Self::RecordingExported => "RecordingExported",
            Self::Logout => "Logout",
            Self::LoginCompleted => "LoginCompleted",
            Self::ServerChanged => "ServerChanged",
            Self::AnalyticsToggled => "AnalyticsToggled",
            Self::OnboardingCompleted => "OnboardingCompleted",
            Self::NoiseGateToggled => "NoiseGateToggled",
            Self::VoiceModeChanged => "VoiceModeChanged",
            Self::WebsocketServerToggled => "WebsocketServerToggled",
            Self::ChannelJoined => "ChannelJoined",
            Self::ChannelLeft => "ChannelLeft",
            Self::RecordingStarted => "RecordingStarted",
            Self::RecordingStopped => "RecordingStopped",
        }
    }
}
