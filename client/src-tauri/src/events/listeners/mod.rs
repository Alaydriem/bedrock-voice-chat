use tauri::App;
mod change_audio_device_listener;
mod listener_trait;
mod stop_audio_device_listener;

pub(crate) fn register(app: &mut App) {
    stop_audio_device_listener::StopAudioDeviceListener::new(app).listen();
    change_audio_device_listener::ChangeAudioDeviceListener::new(app).listen();
}
