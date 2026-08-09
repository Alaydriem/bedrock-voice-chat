/**
 * The session's one `AudioSettingsManager`, published by the dashboard layout.
 *
 * Context rather than a fresh instance per consumer: the settings cover sits over the dashboard,
 * so the jukebox chip in the header and the Audio pane behind it can both be mounted at once.
 * Two instances would hold two sets of stores and visibly disagree about the same setting.
 */
export const AUDIO_SETTINGS_KEY = Symbol("bvc:audio-settings");
