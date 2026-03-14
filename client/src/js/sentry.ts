import * as Sentry from "@sentry/browser";

export class SentryManager {
    static initialize(): void {
        const dsn = import.meta.env.SENTRY_DSN as string | undefined;
        if (!dsn) return;

        Sentry.init({
            dsn,
            environment: import.meta.env.MODE,
        });
    }
}
