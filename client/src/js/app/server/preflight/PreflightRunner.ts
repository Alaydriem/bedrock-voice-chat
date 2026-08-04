import { invoke } from '@tauri-apps/api/core';
import { warn } from '@tauri-apps/plugin-log';
import type { ApiConfigCheckResponse } from '../../../bindings/ApiConfigCheckResponse';
import type { LoginResponse } from '../../../bindings/LoginResponse';
import type { ServerReachability } from '../../../bindings/ServerReachability';
import { PublicServerConfig } from '../../services/PublicServerConfig';
import type { PreflightOutcome } from './PreflightOutcome';
import type { PreflightStep } from './PreflightStep';
import { PREFLIGHT_STEPS } from './PreflightStepName';
import type { PreflightStepState } from './PreflightStepState';

/** Called after every change to the step list, so a plate can resolve as it goes. */
export type PreflightObserver = (steps: readonly PreflightStep[]) => void;

/**
 * The four checks a server has to pass before it is worth connecting to.
 *
 * Sequential within one server and concurrent across servers, which is why a list resolves
 * in a ragged order rather than top to bottom.
 *
 * A check is skipped when something it needed did not happen, not merely because something
 * before it failed — so a protocol mismatch still measures the UDP path, whose port list came
 * from the handshake rather than from the verdict. Skipped checks say so, because a reader
 * left at "pending" is waiting for a result that is not coming.
 *
 * Certificate expiry is checked and never named. It is the mechanism behind "you are not
 * signed in any more", and naming it asks a player to understand mTLS to work out that they
 * need to sign in again.
 */
export class PreflightRunner {
    /** Above this, the round trip is worth saying out loud rather than just recording. */
    static readonly SLOW_MS = 120;

    private static readonly STANDARD_QUIC_PORT = 443;

    private readonly steps: PreflightStep[];
    private readonly observer: PreflightObserver;

    constructor(observer: PreflightObserver) {
        this.observer = observer;
        this.steps = PreflightRunner.pending();
    }

    static pending(): PreflightStep[] {
        return PREFLIGHT_STEPS.map((name) => ({ name, state: 'pending', note: '', ms: 0 }));
    }

    async run(server: string): Promise<PreflightOutcome> {
        this.emit();

        const credentials = await this.credentials(server);
        if (!credentials) {
            this.skipFrom(1);
            return PreflightRunner.blank('reauth');
        }

        const handshake = await this.handshake(server, credentials);
        if (!handshake) {
            // The server refusing a certificate and the server not being there both fail
            // here, and they lead to different places: one to a sign-in, one to whoever
            // runs it. Asking the one question a server answers to anybody settles it —
            // and the answer carries the port list, so the UDP path is still measurable.
            const answered = await PublicServerConfig.read(server).catch(() => null);
            this.note(1, answered ? 'server refused these credentials' : 'no response');

            // Protocol needs the authenticated response. Nothing produced one.
            this.skipFrom(2, 2);

            if (!answered) {
                this.skipFrom(3);
                return PreflightRunner.blank('unreachable');
            }

            const quic = await this.quicPath(server, answered.quic_port, answered.quic_ports);
            return { ...PreflightRunner.blank('reauth'), quicPort: quic.port };
        }

        const { response, rtt } = handshake;
        const measured = {
            rtt,
            slow: rtt > PreflightRunner.SLOW_MS,
            serverVersion: response.config.protocol_version,
            clientVersion: response.client_version,
            clientTooOld: response.client_too_old,
        };

        /*
         * The protocol check does not gate the UDP path. A check is skipped when something
         * it needed did not happen, not merely because something before it failed — and the
         * UDP path needs the port list, which the handshake produced.
         *
         * It is also the case where the measurement is worth the most: a blocked client
         * cannot connect and find out for itself, so this is the only place the answer is
         * available at all.
         */
        const compatible = this.protocol(response);
        const quic = await this.quicPath(
            server,
            response.config.quic_port,
            response.config.quic_ports,
        );

        // First failing check wins, and the protocol check runs before this one.
        const status = !compatible ? 'version_mismatch' : quic.open ? 'connect' : 'udp_blocked';

        return { status, ...measured, quicPort: quic.port };
    }

    /**
     * Step one. A missing sign-in and an expired one are the same answer — sign in again —
     * so they are the same failure with the same wording.
     */
    private async credentials(server: string): Promise<LoginResponse | null> {
        const done = this.begin(0);
        try {
            const credentials = await invoke<LoginResponse>('get_credentials', { server });
            if (await invoke<boolean>('is_certificate_expired', { server })) {
                done('bad', 'no valid sign-in for this server');
                return null;
            }
            done('ok', `signed in as ${credentials.gamertag}`);
            return credentials;
        } catch (e) {
            warn(`Preflight credentials failed for ${server}: ${e}`);
            done('bad', 'no valid sign-in for this server');
            return null;
        }
    }

    /** Step two, and the only round trip this screen measures. */
    private async handshake(
        server: string,
        credentials: LoginResponse,
    ): Promise<{ response: ApiConfigCheckResponse; rtt: number } | null> {
        const done = this.begin(1);
        const started = performance.now();
        try {
            await invoke('api_pool_client', {
                endpoint: server,
                cert: credentials.certificate_ca,
                pem: credentials.certificate + credentials.certificate_key,
            });
            const response = await invoke<ApiConfigCheckResponse>('api_get_config', { server });
            const rtt = Math.max(1, Math.round(performance.now() - started));
            done(
                rtt > PreflightRunner.SLOW_MS ? 'warn' : 'ok',
                `mTLS · TLS 1.3 · ${rtt} ms round trip`,
            );
            return { response, rtt };
        } catch (e) {
            warn(`Preflight handshake failed for ${server}: ${e}`);
            done('bad', '');
            return null;
        }
    }

    /** Step three. Both directions are a mismatch; only one of them an update can fix. */
    private protocol(response: ApiConfigCheckResponse): boolean {
        const done = this.begin(2);
        const client = response.client_version;
        const server = response.config.protocol_version;

        if (!response.compatible) {
            const which = response.client_too_old ? 'client is too old' : 'server is too old';
            done('bad', `client ${client} · server ${server} — ${which}`);
            return false;
        }

        done('ok', `${client} · server ${server}`);
        return true;
    }

    /**
     * Step four, the one the other three cannot see. All of them ran over TCP 443, and a
     * network that permits HTTPS while dropping UDP passes every one.
     */
    private async quicPath(
        server: string,
        advertised: number,
        ports: number[],
    ): Promise<{ open: boolean; port: number }> {
        const done = this.begin(3);
        try {
            const report = await invoke<ServerReachability>('probe_server', {
                server,
                quicPorts: ports,
                quicPort: advertised,
            });

            if (report.verdict === 'Ready') {
                const port = PreflightRunner.answeringPort(report) ?? advertised;
                const ms = Math.round((report.best_rtt_micros ?? 0) / 1000);
                const fallback = port === PreflightRunner.STANDARD_QUIC_PORT ? '' : ' · fallback port';
                done('ok', `udp/${port} open · ${ms} ms${fallback}`);
                return { open: true, port };
            }

            const probes = report.quic.length || 1;
            done(
                'bad',
                report.verdict === 'NoRoute'
                    ? `no route to udp/${advertised} from this device`
                    : `udp/${advertised} unreachable · ${probes} ${probes === 1 ? 'probe' : 'probes'}, no response`,
            );
            return { open: false, port: advertised };
        } catch (e) {
            warn(`Preflight QUIC probe failed for ${server}: ${e}`);
            done('bad', `udp/${advertised} could not be probed`);
            return { open: false, port: advertised };
        }
    }

    private static answeringPort(report: ServerReachability): number | null {
        for (const endpoint of report.quic) {
            const outcome = endpoint.outcome;
            if (outcome.state === 'answered' && outcome.rtt_micros === report.best_rtt_micros) {
                return endpoint.port;
            }
        }
        return null;
    }

    /**
     * Mark a step running, and hand back the function that records what it found. The step's
     * duration is measured here rather than reported by each check, so every row on the
     * panel is timed the same way.
     */
    private begin(index: number): (state: PreflightStepState, note: string) => void {
        const started = performance.now();
        this.set(index, { state: 'running', note: '', ms: 0 });
        return (state, note) => {
            this.set(index, { state, note, ms: Math.max(1, Math.round(performance.now() - started)) });
        };
    }

    private note(index: number, note: string): void {
        this.set(index, { note });
    }

    /**
     * Mark checks as never having run.
     *
     * Named for what it says on the panel rather than for stopping: a check is skipped when
     * something it needed did not happen, which is not the same as something before it
     * having failed.
     */
    private skipFrom(from: number, to: number = this.steps.length - 1): void {
        for (let i = from; i <= to; i++) {
            this.set(i, { state: 'skipped', note: 'not run', ms: 0 });
        }
    }

    /** An outcome with nothing measured, for a preflight that never reached the server. */
    private static blank(status: PreflightOutcome['status']): PreflightOutcome {
        return {
            status,
            rtt: 0,
            slow: false,
            quicPort: PreflightRunner.STANDARD_QUIC_PORT,
            serverVersion: '',
            clientVersion: '',
            clientTooOld: false,
        };
    }

    private set(index: number, changes: Partial<PreflightStep>): void {
        this.steps[index] = { ...this.steps[index], ...changes };
        this.emit();
    }

    private emit(): void {
        this.observer([...this.steps]);
    }
}
