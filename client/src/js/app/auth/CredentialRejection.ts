import { warn } from "@charlesportwoodii/tauri-plugin-curia";

/**
 * The answer a background API call gives once the server has confirmed it no longer accepts
 * this device's certificate. By then Rust has already forgotten the server.
 *
 * A redirect rather than a Tauri event: on Android a `listen()` registration can be lost
 * without any error, and this is the one notice that must not be.
 */
export class CredentialRejection {
    static readonly MARKER = "CERT_INVALID";

    /** True when `err` is that answer, in which case the page has been sent to CERT01. */
    static handle(err: unknown): boolean {
        const message = err instanceof Error ? err.message : String(err);
        if (!message.includes(CredentialRejection.MARKER)) return false;

        void warn("server rejected this device's certificate; leaving the dashboard");
        // Replaced, not pushed: back must not return to a dashboard for a forgotten server.
        window.location.replace("/error?code=CERT01");
        return true;
    }
}
