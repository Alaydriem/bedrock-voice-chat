use std::fmt::Write;
use std::net::IpAddr;

use common::structs::audio::NoiseGateStatus;
use common::structs::metrics::{LinkDiagnosticsSnapshot, LinkSample};

// The copyable support artifact.
//
// Reports arrive conversationally — "he sounds choppy" — not timed to a log window, so this
// renders the same facts as the log line plus the per-speaker table and the recent trend, in a
// form that survives being pasted into a chat message.
pub struct DiagnosticsReport;

impl DiagnosticsReport {
    pub fn render(snapshot: &LinkDiagnosticsSnapshot) -> String {
        let mut out = String::new();

        let _ = writeln!(out, "BVC link diagnostics");
        let _ = writeln!(out, "====================");
        let _ = writeln!(out);

        Self::render_session(&mut out, snapshot);
        Self::render_link(&mut out, snapshot);
        Self::render_devices(&mut out, snapshot);
        Self::render_peers(&mut out, snapshot);
        Self::render_trend(&mut out, &snapshot.history);

        out
    }

    // Reads "disconnected" rather than printing a wall of zeros, which would look like a
    // flawless link to anyone reading the paste.
    pub fn render_disconnected() -> String {
        "BVC link diagnostics\n====================\n\nNot connected. Nothing to report.\n"
            .to_string()
    }

    fn render_session(out: &mut String, snapshot: &LinkDiagnosticsSnapshot) {
        let s = &snapshot.session;
        let _ = writeln!(out, "Session");
        let _ = writeln!(
            out,
            "  Server            {}",
            Self::display_host(s.server.as_deref())
        );
        let _ = writeln!(
            out,
            "  Protocol          {}",
            s.protocol_version.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(
            out,
            "  Proximity range   {}",
            s.proximity_range
                .map(|v| format!("{v} m"))
                .unwrap_or_else(|| "unknown".to_string())
        );
        let _ = writeln!(
            out,
            "  Falloff           {}",
            s.falloff.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(
            out,
            "  Family preference {}",
            s.family_preference
                .map(|p| format!("{p:?}"))
                .unwrap_or_else(|| "unknown".to_string())
        );
        let _ = writeln!(out);
    }

    fn render_link(out: &mut String, snapshot: &LinkDiagnosticsSnapshot) {
        let l = &snapshot.link;
        let _ = writeln!(out, "Link");
        let _ = writeln!(out, "  State             {}", l.state);
        let _ = writeln!(out, "  Uptime            {}", Self::duration(l.uptime_secs));
        let _ = writeln!(
            out,
            "  Round trip        {}",
            l.rtt_ms
                .map(|v| format!("{v} ms"))
                .unwrap_or_else(|| "unmeasured".to_string())
        );
        let _ = writeln!(
            out,
            "  Uplink loss       {:.1} %  (packets we sent that were declared lost)",
            l.uplink_loss_pct
        );
        let _ = writeln!(
            out,
            "  Downlink loss     {}",
            l.downlink_loss_pct
                .map(|v| format!("{v:.1} %  (from the server's own per-connection sequence)"))
                .unwrap_or_else(|| {
                    "unmeasured  (server predates the sequence field)".to_string()
                })
        );
        let _ = writeln!(
            out,
            "  Burst loss        {:.1} %  (QUIC packet-number runs; a lower bound)",
            l.burst_loss_pct
        );
        let _ = writeln!(
            out,
            "  Worst concealment {:.1} %  (fraction of one speaker's audio that was fabricated)",
            l.worst_concealment_pct
        );
        let _ = writeln!(
            out,
            "  Jitter buffer     {} ms / {} drops",
            l.jitter_buffer_ms, l.jitter_buffer_drops
        );
        let _ = writeln!(
            out,
            "  QUIC port         {}",
            l.quic_port
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        let _ = writeln!(
            out,
            "  Address family    {}",
            l.family
                .map(|f| format!("{f:?}"))
                .unwrap_or_else(|| "unknown".to_string())
        );
        let _ = writeln!(out, "  Paths used        {}", l.paths_used);
        let _ = writeln!(out, "  Datagrams dropped {}", l.datagrams_dropped);
        let _ = writeln!(out, "  Quality           {:?}", l.quality);
        if l.stalled {
            let _ = writeln!(
                out,
                "  ** Sending, but nothing is coming back. The server has stopped answering \
                 this connection."
            );
        }
        let _ = writeln!(out);
    }

    fn render_devices(out: &mut String, snapshot: &LinkDiagnosticsSnapshot) {
        let m = &snapshot.mic;
        let p = &snapshot.playback;
        let _ = writeln!(out, "Your mic");
        let _ = writeln!(
            out,
            "  Device            {}",
            m.device.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(out, "  Sample rate       {}", Self::hz(m.sample_rate));
        let _ = writeln!(
            out,
            "  Noise gate        {}",
            match m.noise_gate {
                NoiseGateStatus::Disabled => "off (not in the audio path)",
                NoiseGateStatus::Open => "on, open (passing audio)",
                NoiseGateStatus::Closed => "on, closed (cutting the mic)",
            }
        );
        let _ = writeln!(out, "  Muted             {}", m.muted);
        let _ = writeln!(
            out,
            "  Capturing         {}",
            match m.capture_frames_per_sec {
                None => "not measured yet".to_string(),
                Some(rate) if rate == 0.0 =>
                    "0 frames/s  <- the capture device is delivering nothing".to_string(),
                Some(rate) => format!("{rate:.0} frames/s"),
            }
        );
        let _ = writeln!(
            out,
            "  Sending           {:.0} audio datagrams/s",
            m.datagrams_per_sec
        );
        let _ = writeln!(out);

        let _ = writeln!(out, "Interface");
        let _ = writeln!(
            out,
            "  Meter updates     {:.1}/s  (messages this client sends its own UI for the meters)",
            snapshot.meter_events_per_sec
        );
        let _ = writeln!(out);

        let _ = writeln!(out, "What you hear");
        let _ = writeln!(
            out,
            "  Device            {}",
            p.device.as_deref().unwrap_or("unknown")
        );
        let _ = writeln!(out, "  Sample rate       {}", Self::hz(p.sample_rate));
        let _ = writeln!(
            out,
            "  Receiving         {:.0} datagrams/s",
            p.datagrams_per_sec
        );
        let _ = writeln!(out, "  Deafened          {}", p.deafened);
        let _ = writeln!(out, "  Muted by you      {}", p.muted_peer_count);
        let _ = writeln!(out);
    }

    // The table that makes chop attributable. Underruns with no drops means that speaker's
    // client stopped emitting; drops mean the network between them and here.
    fn render_peers(out: &mut String, snapshot: &LinkDiagnosticsSnapshot) {
        let _ = writeln!(out, "Per speaker");
        if snapshot.peers.is_empty() {
            let _ = writeln!(out, "  (nobody heard in this window)");
            let _ = writeln!(out);
            return;
        }

        let _ = writeln!(
            out,
            "  {:<20} {:>9} {:>9} {:>9} {:>7} {:>8} {:>9} {:>8}",
            "speaker", "underrun", "overflow", "ooo", "plc", "silence", "conceal%", "buffer"
        );
        for peer in &snapshot.peers {
            let _ = writeln!(
                out,
                "  {:<20} {:>9} {:>9} {:>9} {:>7} {:>8} {:>9.1} {:>6} ms",
                Self::truncate(&peer.name, 20),
                peer.underruns,
                peer.overflow_drops,
                peer.ooo_drops,
                peer.plc_frames,
                peer.silence_frames,
                peer.concealment_pct,
                peer.buffer_ms,
            );
        }
        let _ = writeln!(out);
    }

    // Without a trend, "it was fine thirty seconds ago" is unanswerable from a paste.
    fn render_trend(out: &mut String, history: &[LinkSample]) {
        let _ = writeln!(out, "Round trip, most recent last");
        if history.is_empty() {
            let _ = writeln!(out, "  (no samples)");
            return;
        }

        let rendered: Vec<String> = history
            .iter()
            .map(|s| {
                s.rtt_ms
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string())
            })
            .collect();

        for chunk in rendered.chunks(20) {
            let _ = writeln!(out, "  {}", chunk.join(" "));
        }
    }

    // A v4-mapped address is an IPv4 destination wearing a v6 sockaddr. Printing the mapped
    // form would tell a reader they are on IPv6 when they are not.
    fn display_host(server: Option<&str>) -> String {
        let Some(server) = server else {
            return "unknown".to_string();
        };

        match server.parse::<IpAddr>() {
            Ok(IpAddr::V6(v6)) => match v6.to_ipv4_mapped() {
                Some(v4) => v4.to_string(),
                None => server.to_string(),
            },
            _ => server.to_string(),
        }
    }

    fn hz(rate: Option<u32>) -> String {
        rate.map(|v| format!("{:.1} kHz", v as f32 / 1000.0))
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn duration(secs: u64) -> String {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        if hours > 0 {
            format!("{hours}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes}:{seconds:02}")
        }
    }

    fn truncate(value: &str, max: usize) -> String {
        if value.chars().count() <= max {
            return value.to_string();
        }
        value.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
