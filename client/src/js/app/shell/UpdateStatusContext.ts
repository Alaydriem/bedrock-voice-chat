/**
 * The session's one `UpdateStatus`, published by the dashboard layout.
 *
 * Context rather than a module singleton: the settings screen is a descendant route of the
 * dashboard, so the layout that owns the poller can hand the same object down without it
 * becoming reachable from anywhere in the app.
 */
export const UPDATE_STATUS_KEY = Symbol('bvc:update-status');
