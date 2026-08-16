/**
 * The session's one `SettingsNavigation`, published by the dashboard layout.
 *
 * Context rather than a module singleton: the layout opens settings and the settings
 * routes leave it, so both need the same object without it becoming reachable from
 * anywhere in the app. It also holds whether this session pushed the entries beneath
 * the current screen, which a second instance would answer wrongly.
 */
export const SETTINGS_NAV_KEY = Symbol("bvc:settings-navigation");
