import { describe, expect, it } from "vitest";
import { Diagnostics } from "$radial/core/controllers/Diagnostics";
import { DiagnosticsView } from "../../../js/app/dashboard/DiagnosticsView";
import type { LinkDiagnosticsSnapshot } from "../../../js/bindings/LinkDiagnosticsSnapshot";

function snapshot(
    link: Partial<LinkDiagnosticsSnapshot["link"]> = {},
    session: Partial<LinkDiagnosticsSnapshot["session"]> = {},
): LinkDiagnosticsSnapshot {
    return {
        captured_at_ms: 0n,
        mic: {
            device: "Scarlett 2i2",
            sample_rate: 48000,
            noise_gate: "Open",
            muted: false,
            datagrams_per_sec: 48,
        },
        playback: {
            device: "HD 560S",
            sample_rate: 48000,
            datagrams_per_sec: 47,
            muted_peer_count: 0,
            deafened: false,
        },
        link: {
            state: "connected",
            uptime_secs: 60n,
            rtt_ms: 40,
            rtt_variance_ms: 4,
            uplink_loss_pct: 0,
            downlink_loss_pct: 0,
            burst_loss_pct: 0,
            worst_concealment_pct: 0,
            jitter_buffer_ms: 40,
            jitter_buffer_drops: 0n,
            quic_port: 443,
            family: null,
            paths_used: 1,
            datagrams_dropped: 0n,
            stalled: false,
            quality: "Good",
            ...link,
        },
        session: {
            server: "voice.example.com",
            protocol_version: "2.1.0",
            proximity_range: 48,
            falloff: "inverse-square",
            family_preference: null,
            transport: "Quic",
            ...session,
        },
        peers: [],
        history: [],
    } as never as LinkDiagnosticsSnapshot;
}

const extra = { reconnecting: false, pttIdle: false, visiblePlayers: 3 };

describe("DiagnosticsView", () => {
    // An unmeasured downlink must not fabricate a verdict, and must not suppress one either.
    it("treats an unmeasured downlink as unmeasured rather than as zero loss", () => {
        const rows = DiagnosticsView.extraGroups(
            snapshot({ downlink_loss_pct: null, uplink_loss_pct: 2 }),
            { connected: true, lastFrameAgoMs: 100, attempts: 0 },
        );
        const loss = rows.find((group) => group.title === "Loss, by direction");

        expect(loss?.rows).toContainEqual(["Downlink", "unmeasured (server too old)"]);
        expect(loss?.rows).toContainEqual(["Worst direction", "2 %"]);
    });

    it("headlines the worse of the two directions", () => {
        expect(
            DiagnosticsView.worstLoss(snapshot({ uplink_loss_pct: 1, downlink_loss_pct: 6 })),
        ).toBe(6);
    });

    // A lower bound from QUIC's own packet numbers. Counting it in the verdict would read the
    // same loss as two separate problems.
    it("keeps the provable lower bound out of the headline figure", () => {
        expect(
            DiagnosticsView.worstLoss(
                snapshot({ uplink_loss_pct: 0, downlink_loss_pct: 0, burst_loss_pct: 9 }),
            ),
        ).toBe(0);
    });

    it("renders an absent device name rather than an empty one", () => {
        const input = DiagnosticsView.input(
            { ...snapshot(), mic: { ...snapshot().mic, device: null } } as never,
            extra,
        );

        expect(input.inputDevice).toBe("—");
    });

    it("skips unmeasured round trips out of the scope history rather than plotting zeros", () => {
        const withHistory = {
            ...snapshot(),
            history: [
                { at_ms: 1n, rtt_ms: 30, uplink_loss_pct: 0, worst_concealment_pct: 0 },
                { at_ms: 2n, rtt_ms: null, uplink_loss_pct: 0, worst_concealment_pct: 0 },
                { at_ms: 3n, rtt_ms: 44, uplink_loss_pct: 0, worst_concealment_pct: 0 },
            ],
        } as never as LinkDiagnosticsSnapshot;

        // A zero would draw as an impossibly perfect round trip.
        expect(DiagnosticsView.history(withHistory)).toEqual([30, 44]);
    });
});

describe("Diagnostics.verdict", () => {
    // Nothing else on the panel reveals this: the microphone is fine, the link reads as up,
    // and every other number looks healthy.
    it("puts a stall above loss, because a stall means nobody hears you at all", () => {
        const input = DiagnosticsView.input(
            snapshot({ stalled: true, uplink_loss_pct: 20 }),
            extra,
        );

        const verdict = Diagnostics.verdict(input);
        expect(verdict.severity).toBe("bad");
        expect(verdict.code).toBe("stalled");
    });

    it("still puts reconnecting above a stall", () => {
        const input = DiagnosticsView.input(snapshot({ stalled: true }), {
            ...extra,
            reconnecting: true,
            attempt: 2,
        });

        expect(Diagnostics.verdict(input).code).toBe("reconnecting");
    });

    // Concealment is what a listener actually experienced; loss is only its cause.
    it("reports what was heard above the loss that caused it", () => {
        const input = DiagnosticsView.input(
            snapshot({ worst_concealment_pct: 12, uplink_loss_pct: 4 }),
            extra,
        );

        expect(Diagnostics.verdict(input).code).toBe("concealment");
    });

    it("says everything is fine when it is", () => {
        expect(Diagnostics.verdict(DiagnosticsView.input(snapshot(), extra))).toEqual({
            severity: "ok",
            code: "fine",
        });
    });

    it("passes the transport through from the session", () => {
        const input = DiagnosticsView.input(snapshot({}, { transport: "WebSocket" }), extra);

        expect(input.transport).toBe("WebSocket");
    });

    // A server too old to report one, or nothing connected. Defaulting to QUIC would state a
    // fact the backend did not report.
    it("reports an absent transport as absent", () => {
        const input = DiagnosticsView.input(snapshot({}, { transport: null }), extra);

        expect(input.transport).toBeNull();
    });
});

describe("DiagnosticsView.channelRow", () => {
    it("names a channel that never opened", () => {
        const [, value] = DiagnosticsView.channelRow({
            connected: false,
            lastFrameAgoMs: null,
            attempts: 3,
        });
        expect(value).toContain("not connected");
        expect(value).toContain("3");
    });

    it("reports a live channel with the age of its last frame", () => {
        const [, value] = DiagnosticsView.channelRow({
            connected: true,
            lastFrameAgoMs: 400,
            attempts: 0,
        });
        expect(value).toContain("connected");
    });

    // Connected with nothing arriving is the state that used to be indistinguishable from a
    // muted microphone, so it gets its own wording rather than reading as healthy.
    it("distinguishes connected-but-silent from connected-and-carrying", () => {
        const [, value] = DiagnosticsView.channelRow({
            connected: true,
            lastFrameAgoMs: 30_000,
            attempts: 0,
        });
        expect(value).toContain("nothing has arrived");
    });
});
