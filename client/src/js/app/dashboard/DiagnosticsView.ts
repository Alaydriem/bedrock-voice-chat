import type { DiagnosticsInput, KvGroup } from '$radial/core/controllers/Diagnostics';
import { Diagnostics } from '$radial/core/controllers/Diagnostics';
import type { MeterProbeSnapshot } from '$radial/core/canvas/MeterProbe';
import type { LinkDiagnosticsSnapshot } from '../../bindings/LinkDiagnosticsSnapshot';
import type { VoiceMode } from '$radial/core/controllers/SelfState';
import type { VoiceDiagnostics } from './SelfController';

/**
 * The backend's snapshot, as the status panel's view type.
 *
 * Adapts rather than widens: the kit's `DiagnosticsInput` is a view type with concrete
 * numbers, and the backend reports several of them as absent-or-present. Absence is real —
 * a port before the connection settles, a device name the host will not give up, a downlink
 * loss figure a server too old to stamp sequences cannot produce — and it has to read as
 * unmeasured rather than as zero, which would draw as a flawless link.
 */
export class DiagnosticsView {
    /** Neither zero nor a guess. */
    private static readonly UNKNOWN = '—';

    static input(
        snapshot: LinkDiagnosticsSnapshot,
        extra: {
            reconnecting: boolean;
            attempt?: number;
            pttIdle: boolean;
            visiblePlayers: number;
        },
    ): DiagnosticsInput {
        const { mic, playback, link, session } = snapshot;

        return {
            rtt: link.rtt_ms ?? 0,
            lossPercent: DiagnosticsView.worstLoss(snapshot),
            jitterMs: link.jitter_buffer_ms,
            jitterDrops: Number(link.jitter_buffer_drops),
            datagramsIn: Math.round(playback.datagrams_per_sec),
            datagramsOut: Math.round(mic.datagrams_per_sec),
            capturing: mic.capture_frames_per_sec ?? null,
            inputDevice: mic.device ?? DiagnosticsView.UNKNOWN,
            inputRate: mic.sample_rate ?? Diagnostics.EXPECTED_RATE,
            outputDevice: playback.device ?? DiagnosticsView.UNKNOWN,
            outputRate: playback.sample_rate ?? Diagnostics.EXPECTED_RATE,
            port: link.quic_port ?? 443,
            transport: session.transport ?? null,
            protocol: session.protocol_version ?? DiagnosticsView.UNKNOWN,
            rangeMetres: session.proximity_range ?? 0,
            falloff: session.falloff ?? DiagnosticsView.UNKNOWN,
            server: session.server ?? DiagnosticsView.UNKNOWN,
            uptimeSeconds: Number(link.uptime_secs),
            reconnecting: extra.reconnecting,
            attempt: extra.attempt,
            stalled: link.stalled,
            concealmentPercent: DiagnosticsView.round(link.worst_concealment_pct),
            muted: mic.muted,
            noiseGate: mic.noise_gate,
            deafened: playback.deafened,
            pttIdle: extra.pttIdle,
            mutedOthers: playback.muted_peer_count,
            visiblePlayers: extra.visiblePlayers,
        };
    }

    /**
     * The worse of the two directions, which is what a verdict should headline.
     *
     * An unmeasured downlink contributes nothing rather than a zero: it must not fabricate a
     * verdict, and it must not suppress one either.
     */
    static worstLoss(snapshot: LinkDiagnosticsSnapshot): number {
        const uplink = snapshot.link.uplink_loss_pct;
        const downlink = snapshot.link.downlink_loss_pct ?? 0;
        return DiagnosticsView.round(Math.max(uplink, downlink));
    }

    /**
     * What the microphone is actually doing, as the backend reports it.
     *
     * Three of these four rows exist because nothing else on screen can show them. The mic
     * button never draws the muted glyph in push-to-talk, so mute is invisible there; and a
     * muted input and a capture stream that stopped emitting produce the same flat meter,
     * so the event rate is the only thing that separates "muted" from "not running".
     */
    static voiceGroup(
        voice: VoiceDiagnostics | null,
        uiMode?: VoiceMode,
        capturing?: number | null,
        meter?: MeterProbeSnapshot,
    ): KvGroup {
        if (!voice) {
            return { title: 'Voice', rows: [['State', 'not read yet']] };
        }
        if (!voice.backend) {
            return {
                title: 'Voice',
                rows: [['State', `could not read  ← ${voice.error ?? 'no reason given'}`]],
            };
        }

        const { backend, mic } = voice;
        const ptt = backend.voiceMode === 'pushToTalk';
        // The mode reaches the button by event, and the button is what turns a tap into a
        // hold. Disagreement here means that event did not arrive, which is invisible
        // otherwise: the button simply keeps behaving like the mode it last heard about.
        const agreed = uiMode === undefined || (uiMode === 'ptt') === ptt;
        const mode = ptt ? 'push-to-talk' : 'open mic';
        return {
            title: 'Voice',
            rows: [
                [
                    'Mode',
                    agreed
                        ? mode
                        : `${mode}  ← the button still thinks ${uiMode === 'ptt' ? 'push-to-talk' : 'open mic'}`,
                ],
                [
                    'Microphone',
                    backend.inputMuted
                        ? `muted${ptt ? '  (resting state of push-to-talk)' : '  ← nothing is being captured'}`
                        : 'open',
                ],
                ['Hold', ptt ? (backend.pttActive ? 'held' : 'released') : 'n/a in open mic'],
                ['Capture stream', DiagnosticsView.captureStream(mic, capturing ?? null)],
                ...(meter ? [['Self meter', DiagnosticsView.selfMeter(meter)] as [string, string]] : []),
            ],
        };
    }

    /**
     * What the pill drew against what it was handed.
     *
     * The capture-stream row above ends at the level feed; this row covers the last hop, which
     * has now failed on its own: a phone received a full stream of levels and painted none of
     * them, and nothing on screen could say which side was lying. The probe counts both sides
     * inside the binding, so the gap is a measurement rather than an inference.
     */
    private static readonly METER_STALL_MS = 2000;

    private static selfMeter(meter: MeterProbeSnapshot): string {
        if (!meter.mounted) {
            return 'not mounted  ← no pill has registered a meter in this window';
        }
        if (meter.levels === 0) {
            return 'no levels have reached it  ← the feed above decides whether that is a fault';
        }
        if (meter.paints === 0) {
            return `${meter.levels} levels arrived and none were painted  ← the renderer is not drawing them`;
        }
        const levelsFresh =
            meter.levelAgeMs !== null && meter.levelAgeMs < DiagnosticsView.METER_STALL_MS;
        const paintsStale =
            meter.paintAgeMs === null || meter.paintAgeMs > DiagnosticsView.METER_STALL_MS;
        if (levelsFresh && paintsStale) {
            return `stopped painting ${Math.round((meter.paintAgeMs ?? 0) / 1000)}s ago  ← levels are still arriving`;
        }
        const age =
            meter.paintAgeMs === null ? '' : `, ${Math.round(meter.paintAgeMs / 1000)}s ago`;
        return `painting — last level ${meter.lastLevel.toFixed(2)}${age}`;
    }

    /**
     * Whether level events are reaching this window, told apart from whether anything is
     * being captured.
     *
     * A muted stream still emits, at rms 0, so the meter draws a muted microphone and a dead
     * one identically and the event count is the only thing between them.
     *
     * What this counts, though, is events arriving *here* — and it used to report their
     * absence as "the capture stream is not running", which is a claim about the backend it
     * cannot make. A phone carrying audio in both directions read as a dead microphone,
     * because the events were not arriving while the capture stream was perfectly alive.
     * `capturing` is the backend's own frame rate off the device, so the two can now be told
     * apart: frames without events is a broken bridge to this window, not a broken microphone.
     */
    private static captureStream(mic: VoiceDiagnostics['mic'], capturing: number | null): string {
        if (!mic.attached) {
            return 'not attached  ← the level listener failed to register in this window';
        }
        // Above the event count, because it is the stronger statement: events arrived and this
        // window could not read them, which no amount of looking at the transport will explain.
        if (mic.failures > 0) {
            return `${mic.failures} of ${mic.events} events could not be read  ← the meter is receiving but failing to handle them`;
        }
        if (mic.events === 0) {
            if (capturing !== null && capturing > 0) {
                return `no events here, but ${Math.round(capturing)} frames/s are being captured  ← the meter is not receiving, the microphone is fine`;
            }
            if (capturing === null) return 'no events  ← nothing captured yet either';
            return 'no events  ← the capture stream is not running';
        }
        // The meter's own level, not an RMS. It was labelled `rms` and stopped being one when
        // levels became quantised steps — and it is the number that says whether a still meter
        // is being told to be still or is failing to draw what it was told.
        const rate = `${mic.eventsPerSecond.toFixed(1)}/s`;
        const level =
            mic.lastRms > 0 ? `level ${mic.lastRms.toFixed(2)}` : 'level 0  ← reported as not speaking';
        const stale = mic.silentForMs !== null && mic.silentForMs > 1000;
        return stale
            ? `${rate}, ${level}  ← stopped ${Math.round(mic.silentForMs! / 1000)}s ago`
            : `${rate}, ${level}`;
    }

    /**
     * The rows the grid shows beyond the kit's own.
     *
     * All four loss figures, each doing one job. `burst_loss_pct` is a provable lower bound
     * from QUIC's packet numbers and stays out of the verdict — counting it there would read
     * the same loss as two separate problems.
     */
    static extraGroups(snapshot: LinkDiagnosticsSnapshot): KvGroup[] {
        const link = snapshot.link;
        const downlink =
            link.downlink_loss_pct === null || link.downlink_loss_pct === undefined
                ? 'unmeasured (server too old)'
                : `${DiagnosticsView.round(link.downlink_loss_pct)} %`;

        return [
            {
                title: 'Loss, by direction',
                rows: [
                    ['Uplink', `${DiagnosticsView.round(link.uplink_loss_pct)} %`],
                    ['Downlink', downlink],
                    ['Provable, lower bound', `${DiagnosticsView.round(link.burst_loss_pct)} %`],
                    ['Worst direction', `${DiagnosticsView.worstLoss(snapshot)} %`],
                ],
            },
            {
                // Not a network figure, and not under any of the subsystems, because it belongs
                // to none of them. On a phone every one of these is a unit of main-thread work
                // on the same thread that paints the meters they feed, so it is the number that
                // decides whether the meters can keep up — and the only way to tell a change
                // that halved the traffic from one that did nothing.
                title: 'Interface',
                rows: [
                    [
                        'Meter updates',
                        `${DiagnosticsView.round(snapshot.meter_events_per_sec)}/s` +
                            (snapshot.meter_events_per_sec === 0 ? '  (nobody is speaking)' : ''),
                    ],
                ],
            },
            {
                title: 'What you heard',
                rows: [
                    // Named as the worst speaker's figure, because that is what it is: the
                    // maximum across everyone audible, not an average over them. Labelled
                    // "Reconstructed" alone it read as the latter, so one person on a bad
                    // connection looked like the whole room breaking up.
                    [
                        'Reconstructed, worst speaker',
                        `${DiagnosticsView.round(link.worst_concealment_pct)} %`,
                    ],
                    ['Quality', String(link.quality).toLowerCase()],
                    ['Address family', link.family ?? DiagnosticsView.UNKNOWN],
                    ['Paths used', String(link.paths_used)],
                ],
            },
        ];
    }

    /** RTT history for the scope, oldest first. Absent samples are skipped, not zeroed. */
    static history(snapshot: LinkDiagnosticsSnapshot): number[] {
        return snapshot.history
            .map((sample) => sample.rtt_ms)
            .filter((rtt): rtt is number => rtt !== null && rtt !== undefined);
    }

    private static round(value: number): number {
        return Math.round(value * 10) / 10;
    }
}
