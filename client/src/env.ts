import { defineEnvVars } from "@sveltejs/kit/env";

export const variables = defineEnvVars({
    // Content hash of the boot preloader's two static files, computed in vite.config.js.
    // app.html appends it to both URLs so a webview drops its cached copy exactly when the
    // files change.
    PUBLIC_PRELOADER_VERSION: { public: true, static: true },
});
