import * as Sentry from "@sentry/browser";
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "./bindings/AppInfo";
import type { DeviceInfo } from "./bindings/DeviceInfo";

export class SentryManager {
    static initialize(): void {
        const dsn = import.meta.env.SENTRY_DSN as string | undefined;
        if (!dsn) return;

        Sentry.init({
            dsn,
            environment: import.meta.env.MODE,
        });

        void SentryManager.applyAppInfoTags();
        void SentryManager.applyIdentity();
    }

    /**
     * A condition worth counting in the field, from code that recovered and has nothing
     * to throw. No-op until `initialize` has run, which mirrors every other capture.
     */
    static warning(message: string): void {
        Sentry.captureMessage(message, "warning");
    }

    /**
     * Reports under `userId` from now on. The platform ID is the Sentry user on both the Rust
     * and the webview side, so a player's session is one user across the two.
     */
    static setUser(userId: string): void {
        Sentry.setUser(userId ? { id: userId } : null);
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

    // The webview's user agent names neither the phone model nor, on iOS, the real OS version.
    // The Rust side reads both from the platform, and this reports the same values.
    private static async applyIdentity(): Promise<void> {
        try {
            SentryManager.setUser(await invoke<string>("get_platform_id"));

            const device = await invoke<DeviceInfo | null>("get_device_info");
            if (!device) return;

            Sentry.setContext("os", {
                name: device.os_name,
                version: device.os_version,
                build: device.os_build,
                kernel_version: device.kernel_version,
            });
            Sentry.setContext("device", {
                manufacturer: device.manufacturer,
                brand: device.brand,
                model: device.model,
                model_id: device.model_id,
                family: device.family,
            });
        } catch (e) {
            Sentry.captureException(e);
        }
    }
}
