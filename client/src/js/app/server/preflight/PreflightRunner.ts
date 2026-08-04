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
 * in a ragged order rather than top to bottom. A failing check stops the sequence: the ones
 * after it never ran and say so, because a reader left at "pending" is waiting for a result
 * that is not coming.
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
        if (!credentials) return this.stop(0, 'reauth');

        const handshake = await this.handshake(server, credentials);
        if (!handshake) {
            // The server refusing a certificate and the server not being there both fail
            // here, and they lead to different places: one to a sign-in, one to whoever
            // runs it. Asking the one question a server answers to anybody settles it.
            const answering = await PublicServerConfig.isAnswering(server);
            this.note(1, answering ? 'server refused these credentials' : 'no response');
            return this.stop(1, answering ? 'reauth' : 'unreachable');
        }

        const { response, rtt } = handshake;
        const slow = rtt > PreflightRunner.SLOW_MS;
        const versions = {
            rtt,
            slow,
            serverVersion: response.config.protocol_version,
            clientVersion: response.client_version,
            clientTooOld: response.client_too_old,
        };
        const advertised = response.config.quic_port;

        if (!this.protocol(response)) {
            return {
                ...this.stop(2, 'version_mismatch'),
                ...versions,
                quicPort: advertised,
            };
        }

        const quic = await this.quicPath(server, response);
        return {
            status: quic.open ? 'connect' : 'udp_blocked',
            ...versions,
            quicPort: quic.port ?? advertised,
        };
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
        response: ApiConfigCheckResponse,
    ): Promise<{ open: boolean; port: number | null }> {
        const done = this.begin(3);
        const advertised = response.config.quic_port;
        try {
            const report = await invoke<ServerReachability>('probe_server', {
                server,
                quicPorts: response.config.quic_ports,
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

    /** Everything after a failure is marked as never having run, and the status stands. */
    private stop(index: number, status: PreflightOutcome['status']): PreflightOutcome {
        for (let i = index + 1; i < this.steps.length; i++) {
            this.set(i, { state: 'skipped', note: 'not run', ms: 0 });
        }
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
