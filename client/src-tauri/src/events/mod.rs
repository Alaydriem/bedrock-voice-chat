mod listener_trait;

use tauri::App;

pub(crate) use listener_trait::ListenerTrait;
use crate::{
    audio::listeners::{
        ChangeAudioDeviceListener,
        StopAudioDeviceListener
    },
    network::listeners::{
        ChangeNetworkStreamListener,
        StopNetworkStreamListener
    }
};

pub(crate) fn register(app: &mut App) {
    StopAudioDeviceListener::new(app).listen();
    ChangeAudioDeviceListener::new(app).listen();
    StopNetworkStreamListener::new(app).listen();
    ChangeNetworkStreamListener::new(app).listen();
}