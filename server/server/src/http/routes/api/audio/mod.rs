pub mod event;
pub mod file;
pub mod stream;

use crate::http::openapi::{RouteSpec, TagDefinition};

inventory::submit! {
    TagDefinition {
        name: "Audio",
        description: "Audio file library and spatial playback events. Upload Ogg/Opus files, \
                      trigger positional audio playback in-game, and manage the audio library.",
    }
}

inventory::submit! {
    RouteSpec {
        prefix: "/api/audio",
        auto_mount: true,
        spec_fn: || {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::openapi_get_routes_spec![settings:
                event::audio_event_play,
                event::audio_event_stop,
                file::audio_file_upload,
                file::audio_file_list,
                file::audio_file_delete,
                file::audio_file_token,
                stream::audio_file_stream
            ]
        },
    }
}
