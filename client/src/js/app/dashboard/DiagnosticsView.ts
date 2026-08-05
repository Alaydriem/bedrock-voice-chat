import type { DiagnosticsInput, KvGroup } from '$radial/core/controllers/Diagnostics';
import { Diagnostics } from '$radial/core/controllers/Diagnostics';
import type { LinkDiagnosticsSnapshot } from '../../bindings/LinkDiagnosticsSnapshot';

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
            inputDevice: mic.device ?? DiagnosticsView.UNKNOWN,
            inputRate: mic.sample_rate ?? Diagnostics.EXPECTED_RATE,
            outputDevice: playback.device ?? DiagnosticsView.UNKNOWN,
            outputRate: playback.sample_rate ?? Diagnostics.EXPECTED_RATE,
            quicPort: link.quic_port ?? 443,
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
