import * as Sentry from "@sentry/browser";
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "./bindings/AppInfo";

export class SentryManager {
    static initialize(): void {
        const dsn = import.meta.env.SENTRY_DSN as string | undefined;
        if (!dsn) return;

        Sentry.init({
            dsn,
            environment: import.meta.env.MODE,
        });

        void SentryManager.applyAppInfoTags();
    }

    /**
     * A condition worth counting in the field, from code that recovered and has nothing
     * to throw. No-op until `initialize` has run, which mirrors every other capture.
     */
    static warning(message: string): void {
        Sentry.captureMessage(message, "warning");
    }

    private static async applyAppInfoTags(): Promise<void> {
        try {
            const info = await invoke<AppInfo>("get_app_info");
            Sentry.setTags({
                version: info.app_version,
                build_number: info.build_number,
            });
        } catch (e) {
            Sentry.captureException(e);
        }
    }
}
