import { invoke } from '@tauri-apps/api/core';
import { warn } from '@tauri-apps/plugin-log';
import { writable, type Readable, type Writable } from 'svelte/store';
import type { RingMode } from '$radial/bindings/RingBinding';
import type { ProtocolCompatibility } from '../../bindings/ProtocolCompatibility';
import type { ServerReachability } from '../../bindings/ServerReachability';
import { PublicServerConfig } from '../services/PublicServerConfig';

export interface ResolveVerdict {
    readonly state: 'editing' | 'ok' | 'bad';
    readonly ring: RingMode;
    /** The line under the field. */
    readonly line: string;
    /** The ring caption, which is upper case by design. */
    readonly caption: string;
}

/**
 * What the address field knows about the address in it.
 *
 * The check runs while the user types rather than on submit, so the ring beside the
 * field is a readout: coloured means confirmed reachable, never merely that something
 * was typed. It goes grey the moment the value changes.
 *
 * It is a readout and nothing more. A probe that fails or times out leaves the field
 * unknown, not invalid, and never blocks signing in.
 *
 * The verdict itself is derived in Rust and arrives on the report, so this screen and
 * the server selector cannot reach different conclusions from the same measurement.
 */
export default class AddressResolver {
    static readonly HOSTNAME = /^[a-z0-9.-]+\.[a-z]{2,}$/i;
    static readonly DEBOUNCE_MS = 700;
    private static readonly STANDARD_QUIC_PORT = 443;

    private static readonly EDITING: ResolveVerdict = {
        state: 'editing',
        ring: 'empty',
        line: '○ Resolving',
        caption: 'RESOLVING',
    };

    private static readonly NO_RESPONSE: ResolveVerdict = {
        state: 'bad',
        ring: 'empty',
        line: '✕ Nothing at that address',
        caption: 'NO RESPONSE',
    };

    private readonly verdictStore: Writable<ResolveVerdict>;
    public readonly verdict: Readable<ResolveVerdict>;

    private timer: ReturnType<typeof setTimeout> | null = null;
    private generation = 0;

    constructor() {
        this.verdictStore = writable(AddressResolver.EDITING);
        this.verdict = { subscribe: this.verdictStore.subscribe };
    }

    /**
     * A reachable server running a protocol this build cannot speak.
     *
     * Deliberately its own state rather than folded into a failure: "nothing at that
     * address" sends someone to check what they typed, and this sends them to an
     * update. Reachability and compatibility are different questions and the answers
     * lead to different places.
     */
    static verdictForIncompatible(check: ProtocolCompatibility): ResolveVerdict {
        const line = check.client_too_old
            ? `✕ Server needs a newer app — it speaks ${check.server_version}, this build speaks ${check.client_version}`
            : `✕ Server is on an older protocol — it speaks ${check.server_version}, this build speaks ${check.client_version}`;

        return {
            state: 'bad',
            ring: 'empty',
            line,
            caption: `PROTOCOL ${check.server_version}`,
        };
    }

    static verdictFor(report: ServerReachability | null): ResolveVerdict {
        if (!report) return AddressResolver.EDITING;

        switch (report.verdict) {
            case 'Ready': {
                const ms = Math.round((report.best_rtt_micros ?? 0) / 1000);
                const port = AddressResolver.answeringPort(report);
                const suffix =
                    port === null || port === AddressResolver.STANDARD_QUIC_PORT
                        ? ''
                        : ` · port ${port}`;
                return {
                    state: 'ok',
                    ring: 'lock',
                    line: `● Resolved · ${ms} ms${suffix}`,
                    caption: `RESOLVED · ${ms} MS`,
                };
            }
            case 'VoiceBlocked':
                return {
                    state: 'bad',
                    ring: 'empty',
                    line: '✕ Server reachable, but no voice path',
                    caption: 'NO VOICE PATH',
                };
            case 'NoRoute':
                return {
                    state: 'bad',
                    ring: 'empty',
                    line: '✕ This device has no route to that address',
                    caption: 'NO ROUTE',
                };
            case 'Unreachable':
                return AddressResolver.NO_RESPONSE;
        }
    }

    /** Which QUIC port produced the winning measurement, for the fallback label. */
    private static answeringPort(report: ServerReachability): number | null {
        const best = report.best_rtt_micros;
        for (const endpoint of report.quic) {
            const outcome = endpoint.outcome as { state: string; rtt_micros?: number };
            if (outcome.state === 'answered' && outcome.rtt_micros === best) {
                return endpoint.port;
            }
        }
        return null;
    }

    /**
     * A new value in the field. Goes grey immediately, then measures once the typing
     * stops: a probe is spent per address, not per keystroke.
     */
    public input(value: string): void {
        this.generation++;
        this.verdictStore.set(AddressResolver.EDITING);
        if (this.timer !== null) clearTimeout(this.timer);

        const trimmed = value
            .trim()
            .replace(/^https?:\/\//, '')
            .replace(/\/$/, '');
        if (!AddressResolver.HOSTNAME.test(trimmed)) return;

        const mine = this.generation;
        this.timer = setTimeout(
            () => void this.measure(trimmed, mine),
            AddressResolver.DEBOUNCE_MS,
        );
    }

    public destroy(): void {
        this.generation++;
        if (this.timer !== null) clearTimeout(this.timer);
        this.timer = null;
    }

    private async measure(host: string, generation: number): Promise<void> {
        const server = `https://${host}`;
        try {
            const config = await PublicServerConfig.read(server);

            // Compatibility is checked before reachability is reported: a server this
            // build cannot speak to is not made better by having a fast voice path,
            // and saying "resolved" first would be a promise the connect cannot keep.
            const compatibility = await invoke<ProtocolCompatibility>(
                'check_protocol_compatibility',
                { serverVersion: config.protocol_version },
            );
            if (generation !== this.generation) return;
            if (!compatibility.compatible) {
                this.verdictStore.set(AddressResolver.verdictForIncompatible(compatibility));
                return;
            }

            const report = await invoke<ServerReachability>('probe_server', {
                server,
                quicPorts: config.quic_ports,
                quicPort: config.quic_port,
            });
            if (generation !== this.generation) return;
            this.verdictStore.set(AddressResolver.verdictFor(report));
        } catch (e) {
            warn(`Address probe failed for ${host}: ${String(e)}`);
            if (generation !== this.generation) return;
            this.verdictStore.set(AddressResolver.NO_RESPONSE);
        }
    }
}
