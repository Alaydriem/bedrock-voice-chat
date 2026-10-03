import { invoke } from "@tauri-apps/api/core";
import { warn } from "@charlesportwoodii/tauri-plugin-curia";
import type { ApiConfigCheckResponse } from "../../../bindings/ApiConfigCheckResponse";
import type { LoginResponse } from "../../../bindings/LoginResponse";
import type { ServerReachability } from "../../../bindings/ServerReachability";
import { PublicServerConfig } from "../../services/PublicServerConfig";
import type { PreflightOutcome } from "./PreflightOutcome";
import type { PreflightStep } from "./PreflightStep";
import { PREFLIGHT_STEPS } from "./PreflightStepName";
import type { PreflightStepState } from "./PreflightStepState";
import type { VoiceTransport } from "./VoiceTransport";

/** Called after every change to the step list, so a plate can resolve as it goes. */
export type PreflightObserver = (steps: readonly PreflightStep[]) => void;

/**
 * The four checks a server has to pass before it is worth connecting to.
 *
 * Sequential within one server and concurrent across servers, which is why a list resolves
 * in a ragged order rather than top to bottom.
 *
 * A check is skipped when something it needed did not happen, not merely because something
 * before it failed — so a protocol mismatch still measures the voice path, whose port list and
 * transport capability came from the handshake rather than from the verdict. Skipped checks say
 * so, because a reader left at "pending" is waiting for a result that is not coming.
 *
 * Certificate expiry is checked and never named. It is the mechanism behind "you are not
 * signed in any more", and naming it asks a player to understand mTLS to work out that they
 * need to sign in again.
 */
export class PreflightRunner {
    /**
     * Above this, the one-way voice delay is worth saying out loud rather than just recording.
     *
     * Voice is judged on the send side only, estimated as half the voice path's round trip.
     * Jitter buffering and Opus framing add roughly another 50–60 ms mouth to ear, which puts
     * this threshold near the ITU-T G.114 150 ms budget for conversation that feels natural.
     */
    static readonly SLOW_SEND_MS = 100;

    private static readonly STANDARD_QUIC_PORT = 443;

    /** What each measured transport means for the plate. */
    private static readonly STATUS_FOR_TRANSPORT: Record<
        VoiceTransport,
        PreflightOutcome["status"]
    > = {
        quic: "connect",
        websocket: "ws_fallback",
        none: "udp_blocked",
    };

    private readonly steps: PreflightStep[];
    private readonly observer: PreflightObserver;

    constructor(observer: PreflightObserver) {
        this.observer = observer;
        this.steps = PreflightRunner.pending();
    }

    static pending(): PreflightStep[] {
        return PREFLIGHT_STEPS.map((name) => ({ name, state: "pending", note: "", ms: 0 }));
    }

    async run(server: string): Promise<PreflightOutcome> {
        this.emit();

        const credentials = await this.credentials(server);
        if (!credentials) {
            this.skipFrom(1);
            return PreflightRunner.blank("reauth");
        }

        const handshake = await this.handshake(server, credentials);
        if (!handshake) {
            // The server refusing a certificate and the server not being there both fail
            // here, and they lead to different places: one to a sign-in, one to whoever
            // runs it. Asking the one question a server answers to anybody settles it —
            // and the answer carries the port list, so the UDP path is still measurable.
            const answered = await PublicServerConfig.read(server).catch(() => null);
            this.note(1, answered ? "server refused these credentials" : "no response");

            // Protocol needs the authenticated response. Nothing produced one.
            this.skipFrom(2, 2);

            if (!answered) {
                this.skipFrom(3);
                return PreflightRunner.blank("unreachable");
            }

            const voice = await this.voicePath(
                server,
                answered.quic_port,
                answered.quic_ports,
                answered.voice_websocket,
            );
            return { ...PreflightRunner.blank("reauth"), quicPort: voice.port };
        }

        const response = handshake;
        const measured = {
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
        const voice = await this.voicePath(
            server,
            response.config.quic_port,
            response.config.quic_ports,
            response.config.voice_websocket,
        );

        // First failing check wins, and the protocol check runs before this one.
        const status = !compatible
            ? "version_mismatch"
            : PreflightRunner.STATUS_FOR_TRANSPORT[voice.transport];

        return {
            status,
            ...measured,
            rtt: voice.rtt,
            slow: PreflightRunner.isSlow(voice.rtt),
            quicPort: voice.port,
        };
    }

    /** Half the round trip is the send side, which is the only half a listener hears. */
    private static isSlow(rtt: number): boolean {
        return rtt / 2 > PreflightRunner.SLOW_SEND_MS;
    }

    /**
     * Step one. A missing sign-in and an expired one are the same answer — sign in again —
     * so they are the same failure with the same wording.
     */
    private async credentials(server: string): Promise<LoginResponse | null> {
        const done = this.begin(0);
        try {
            const credentials = await invoke<LoginResponse>("get_credentials", { server });
            if (await invoke<boolean>("is_certificate_expired", { server })) {
                done("bad", "no valid sign-in for this server");
                return null;
            }
            done("ok", `signed in as ${credentials.gamertag}`);
            return credentials;
        } catch (e) {
            warn(`Preflight credentials failed for ${server}: ${e}`);
            done("bad", "no valid sign-in for this server");
            return null;
        }
    }

    /**
     * Step two. Its duration is TCP, TLS and an HTTP request added together — several round
     * trips, none of them on the voice path — so it is recorded and never judged.
     */
    private async handshake(
        server: string,
        credentials: LoginResponse,
    ): Promise<ApiConfigCheckResponse | null> {
        const done = this.begin(1);
        try {
            await invoke("api_pool_client", {
                endpoint: server,
                cert: credentials.certificate_ca,
                pem: credentials.certificate + credentials.certificate_key,
            });
            const response = await invoke<ApiConfigCheckResponse>("api_get_config", { server });
            done("ok", "mTLS · TLS 1.3");
            return response;
        } catch (e) {
            warn(`Preflight handshake failed for ${server}: ${e}`);
            done("bad", "");
            return null;
        }
    }

    /** Step three. Both directions are a mismatch; only one of them an update can fix. */
    private protocol(response: ApiConfigCheckResponse): boolean {
        const done = this.begin(2);
        const client = response.client_version;
        const server = response.config.protocol_version;

        if (!response.compatible) {
            const which = response.client_too_old ? "client is too old" : "server is too old";
            done("bad", `client ${client} · server ${server} — ${which}`);
            return false;
        }

        done("ok", `${client} · server ${server}`);
        return true;
    }

    /**
     * Step four, the one the other three cannot see. All of them ran over TCP 443, and a
     * network that permits HTTPS while dropping UDP passes every one.
     *
     * It measures both transports because both carry voice. A blocked UDP path used to be
     * the end of the answer; it is now the question the fallback measurement answers, and
     * the difference between the two decides whether this server is connectable at all.
     */
    private async voicePath(
        server: string,
        advertised: number,
        ports: number[],
        voiceWebsocket: boolean,
    ): Promise<{ transport: VoiceTransport; port: number; rtt: number }> {
        const done = this.begin(3);
        try {
            const report = await invoke<ServerReachability>("probe_server", {
                server,
                quicPorts: ports,
                quicPort: advertised,
                voiceWebsocket,
            });

            if (report.verdict === "Ready") {
                const port = PreflightRunner.answeringPort(report.quic, report.best_rtt_micros);
                const resolved = port ?? advertised;
                const ms = Math.round((report.best_rtt_micros ?? 0) / 1000);
                const fallback =
                    resolved === PreflightRunner.STANDARD_QUIC_PORT ? "" : " · fallback port";
                done(
                    PreflightRunner.isSlow(ms) ? "warn" : "ok",
                    `udp/${resolved} open · ~${Math.round(ms / 2)} ms send${fallback}`,
                );
                return { transport: "quic", port: resolved, rtt: ms };
            }

            /*
             * A warning rather than a pass. The check found a working path, which is why the
             * plate offers a connect — and it found it on the transport that costs latency
             * under loss, which is why the row is not green.
             */
            if (report.verdict === "VoiceFallback") {
                const ms = Math.round((report.fallback_rtt_micros ?? 0) / 1000);
                const port =
                    PreflightRunner.answeringPort(report.ws, report.fallback_rtt_micros) ??
                    PreflightRunner.STANDARD_QUIC_PORT;
                done(
                    "warn",
                    `udp/${advertised} blocked · tcp/${port} fallback · ~${Math.round(ms / 2)} ms send`,
                );
                return { transport: "websocket", port: advertised, rtt: ms };
            }

            const probes = report.quic.length || 1;
            done(
                "bad",
                report.verdict === "NoRoute"
                    ? `no route to udp/${advertised} from this device`
                    : `udp/${advertised} unreachable · ${probes} ${probes === 1 ? "probe" : "probes"}, no response`,
            );
            return { transport: "none", port: advertised, rtt: 0 };
        } catch (e) {
            warn(`Preflight voice path probe failed for ${server}: ${e}`);
            done("bad", `udp/${advertised} could not be probed`);
            return { transport: "none", port: advertised, rtt: 0 };
        }
    }

    /** Which endpoint in a leg produced that leg's winning measurement. */
    private static answeringPort(
        endpoints: ServerReachability["quic"],
        best: number | null,
    ): number | null {
        for (const endpoint of endpoints) {
            const outcome = endpoint.outcome;
            if (outcome.state === "answered" && outcome.rtt_micros === best) {
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
        this.set(index, { state: "running", note: "", ms: 0 });
        return (state, note) => {
            this.set(index, {
                state,
                note,
                ms: Math.max(1, Math.round(performance.now() - started)),
            });
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
            this.set(i, { state: "skipped", note: "not run", ms: 0 });
        }
    }

    /** An outcome with nothing measured, for a preflight that never reached the server. */
    private static blank(status: PreflightOutcome["status"]): PreflightOutcome {
        return {
            status,
            rtt: 0,
            slow: false,
            quicPort: PreflightRunner.STANDARD_QUIC_PORT,
            serverVersion: "",
            clientVersion: "",
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
