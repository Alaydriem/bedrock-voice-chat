import { invoke } from '@tauri-apps/api/core';

/** Where the backend's throwaway listeners bound. */
interface ProbePorts {
    ws: number;
    http: number;
}

/** What the listening side observed, which the page cannot see for itself. */
interface ProbeStats {
    tcp_accepted: number;
    ws_upgraded: number;
    http_served: number;
}

/**
 * THROWAWAY. Whether this platform's webview may talk to a loopback socket in this app.
 *
 * Delete this file, `src-tauri/src/spike`, `commands/spike.rs` and the About pane card together.
 *
 * The whole design of the meter pivot rests on one unverified assumption: that a webview at the
 * app's production origin may open `ws://127.0.0.1`. Two platform rules could refuse it, and
 * they need different fixes — Android's cleartext policy, which a release build turns on and a
 * debug build turns off, and iOS App Transport Security, which applies to WebSocket since
 * WKWebView moved to the NSURLSession implementation. A refusal by either arrives in the page as
 * an untyped `error` event and close code 1006, so the page alone cannot name the cause.
 *
 * So the probe reads both sides. The backend counts sockets it accepted; the page counts frames
 * it received. A connection the page never opened and the backend never saw was stopped by a
 * platform policy before it left the process. One the backend accepted and the page still failed
 * is a protocol or handshake problem. That distinction is the entire output.
 *
 * Three hosts, because they do not behave alike: WKWebView has been reported to reach
 * `127.0.0.1` where it fails on `localhost`, and `[::1]` is a separate stack again.
 */
export class LoopbackProbe {
    /** How long a connection attempt is given before it is called a failure. */
    private static readonly ATTEMPT_TIMEOUT_MS = 4000;

    /** Frames to wait for after the socket opens, before calling it good. */
    private static readonly WANT_FRAMES = 2;

    async run(): Promise<string> {
        const lines: string[] = [];
        lines.push('=== BVC loopback WebSocket probe (throwaway) ===');
        lines.push(`when          ${new Date().toISOString()}`);
        lines.push(`origin        ${location.origin}`);
        lines.push(`protocol      ${location.protocol}`);
        lines.push(`href          ${location.href}`);
        lines.push(`secureContext ${String(window.isSecureContext)}`);
        lines.push(`userAgent     ${navigator.userAgent}`);
        lines.push('');

        let ports: ProbePorts;
        try {
            ports = await invoke<ProbePorts>('spike_probe_start');
            lines.push(`listeners     ws=${ports.ws} http=${ports.http}  (bound on 127.0.0.1)`);
        } catch (e) {
            lines.push(`listeners     FAILED TO BIND: ${LoopbackProbe.describe(e)}`);
            lines.push('');
            lines.push('The app could not bind a loopback socket at all. Nothing else was tried.');
            return lines.join('\n');
        }
        lines.push('');

        lines.push(`fetch http://127.0.0.1:${ports.http}/`);
        lines.push(`  ${await this.probeFetch(`http://127.0.0.1:${ports.http}/`)}`);
        lines.push('');

        for (const host of ['127.0.0.1', 'localhost', '[::1]']) {
            const url = `ws://${host}:${ports.ws}/metrics?key=probe`;
            lines.push(`ws    ${url}`);
            lines.push(`  ${await this.probeSocket(url)}`);
        }
        lines.push('');

        lines.push('--- candidate 2: the app\'s own URI scheme (never reaches the network stack) ---');
        for (const url of [
            'http://bvcpush.localhost/probe',
            'bvcpush://localhost/probe',
        ]) {
            lines.push(`push  ${url}`);
            lines.push(`  ${await this.probeFetch(url)}`);
        }
        try {
            const served = await invoke<number>('spike_push_served');
            lines.push(`  backend answered ${served} scheme request(s)`);
        } catch (e) {
            lines.push(`  backend count unreadable: ${LoopbackProbe.describe(e)}`);
        }
        lines.push('');

        try {
            const stats = await invoke<ProbeStats>('spike_probe_stats');
            lines.push(
                `backend saw   tcp_accepted=${stats.tcp_accepted} ws_upgraded=${stats.ws_upgraded} http_served=${stats.http_served}`,
            );
            lines.push('');
            lines.push(
                stats.tcp_accepted === 0
                    ? 'VERDICT ws    nothing reached the listener — a platform policy refused the connection.'
                    : stats.ws_upgraded === 0
                      ? 'VERDICT ws    sockets arrived but no WebSocket handshake completed — a protocol problem, not a policy one.'
                      : 'VERDICT ws    loopback WebSocket works at this origin.',
            );
        } catch (e) {
            lines.push(`backend saw   could not read: ${LoopbackProbe.describe(e)}`);
        }

        return lines.join('\n');
    }

    /**
     * A plain cleartext request to the same loopback address.
     *
     * Cleartext policy refuses this and the WebSocket alike; a WebSocket-specific rule refuses
     * only the socket. Which of the two is refused is the difference between a manifest change
     * and a redesign.
     */
    private async probeFetch(url: string): Promise<string> {
        const started = performance.now();
        try {
            const response = await fetch(url, { cache: 'no-store' });
            const body = await response.text();
            return `ok  status=${response.status} body=${JSON.stringify(body)} in ${LoopbackProbe.since(started)}`;
        } catch (e) {
            return `FAILED  ${LoopbackProbe.describe(e)}  after ${LoopbackProbe.since(started)}`;
        }
    }

    private probeSocket(url: string): Promise<string> {
        return new Promise<string>((resolve) => {
            const started = performance.now();
            let frames = 0;
            let opened = false;
            let settled = false;
            let socket: WebSocket;

            const finish = (verdict: string) => {
                if (settled) return;
                settled = true;
                clearTimeout(timer);
                try {
                    socket.close();
                } catch {
                    // A socket that never opened throws here on some engines; the verdict stands.
                }
                resolve(`${verdict}  after ${LoopbackProbe.since(started)}`);
            };

            const timer = setTimeout(
                () =>
                    finish(
                        opened
                            ? `OPENED but only ${frames} frames arrived  readyState=${socket.readyState}`
                            : `TIMED OUT before opening  readyState=${socket.readyState}`,
                    ),
                LoopbackProbe.ATTEMPT_TIMEOUT_MS,
            );

            try {
                socket = new WebSocket(url);
            } catch (e) {
                clearTimeout(timer);
                resolve(`THREW ON CONSTRUCT  ${LoopbackProbe.describe(e)}`);
                return;
            }

            socket.onopen = () => {
                opened = true;
            };
            socket.onmessage = (event) => {
                frames += 1;
                if (frames >= LoopbackProbe.WANT_FRAMES) {
                    finish(`ok  opened and received ${frames} frames, last=${String(event.data)}`);
                }
            };
            // Engines withhold the reason deliberately, so this records that it happened and
            // leaves the cause to the accept counters.
            socket.onerror = () =>
                finish(
                    opened
                        ? `ERROR after opening  ${frames} frames received`
                        : 'ERROR before opening  (no reason is exposed to script)',
                );
            socket.onclose = (event) =>
                finish(
                    `CLOSED  code=${event.code} reason=${JSON.stringify(event.reason)} clean=${String(event.wasClean)} opened=${String(opened)} frames=${frames}`,
                );
        });
    }

    private static since(started: number): string {
        return `${Math.round(performance.now() - started)}ms`;
    }

    private static describe(e: unknown): string {
        if (e instanceof Error) return `${e.name}: ${e.message}`;
        return String(e);
    }
}
