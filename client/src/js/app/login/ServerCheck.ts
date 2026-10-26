import { invoke } from "@tauri-apps/api/core";
import type { ServerCheckReport } from "../../bindings/ServerCheckReport";

/**
 * The availability check the sign-in screen gates on.
 *
 * Routed through Rust rather than the webview's HTTP plugin so the check and the sign-in
 * that follows it are made over one client configuration: same TLS, same address family,
 * same connect budget. A check made over different settings can answer for a server the
 * sign-in cannot reach.
 *
 * A failed check resolves with its classified reason rather than rejecting; a rejection is a
 * fault in the command, not an answer about the server.
 */
export class ServerCheck {
    public static async report(server: string): Promise<ServerCheckReport> {
        return await invoke<ServerCheckReport>("check_server", { server });
    }
}
