import { Channel, invoke } from "@tauri-apps/api/core";
import type { PositionSocketEvent } from "../../bindings/PositionSocketEvent";

/**
 * A WebSocket that Rust opens.
 *
 * The webview validates certificates against the operating system's roots, which do not
 * include the play roots, so a `wss://` socket to a play-zone server cannot be opened from
 * here. This offers the part of the WebSocket surface the position feed uses, backed by a
 * socket the client opens through its own TLS configuration.
 */
export class RustWebSocket {
    onopen: (() => void) | null = null;
    onmessage: ((event: { data: string }) => void) | null = null;
    onclose: (() => void) | null = null;
    onerror: (() => void) | null = null;

    private id: number | null = null;
    private closed = false;

    constructor(url: string, protocols: string[]) {
        const channel = new Channel<PositionSocketEvent>();
        channel.onmessage = (event) => this.dispatch(event);

        invoke<number>("position_socket_open", { url, protocols, onEvent: channel })
            .then((id) => {
                this.id = id;
                // Closed before Rust answered: the socket exists now and nothing owns it.
                if (this.closed) void invoke("position_socket_close", { id });
            })
            .catch(() => this.onerror?.());
    }

    close(): void {
        this.closed = true;
        if (this.id !== null) void invoke("position_socket_close", { id: this.id });
    }

    private dispatch(event: PositionSocketEvent): void {
        if (this.closed) return;
        switch (event.kind) {
            case "Open":
                this.onopen?.();
                break;
            case "Message":
                this.onmessage?.({ data: event.data });
                break;
            case "Closed":
                this.onclose?.();
                break;
            case "Error":
                this.onerror?.();
                break;
        }
    }
}
