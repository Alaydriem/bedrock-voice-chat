use std::time::Duration;

use crate::bot_proc::BotProc;
use crate::job::AgentJob;
use crate::report::{AgentReport, BotReport};
use crate::tone::Tone;

/// Runs one host's slice of a swarm: spawns each bot as a real `bvc_client_e2e`
/// process, groups them into host-local voice channels, keeps their positions
/// fresh in the server cache, streams a tone for the run window, then collects
/// per-bot delivery counters.
pub struct SwarmAgent {
    bin: String,
    job: AgentJob,
}

impl SwarmAgent {
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
    const SPAWN_STAGGER: Duration = Duration::from_millis(100);

    pub fn new(bin: String, job: AgentJob) -> Self {
        Self { bin, job }
    }

    fn host_label(&self) -> String {
        format!("offset{}", self.job.offset)
    }

    // Bots chunked into host-local groups of `group_size`. Same-channel members
    // hear each other regardless of distance, so groups are self-contained here.
    fn channel_name(&self, group_index: usize) -> String {
        format!("swarm-{}-{}", self.job.offset, group_index)
    }

    pub async fn run(self) -> Result<AgentReport, anyhow::Error> {
        let group_size = self.job.group_size.max(1);
        let codes = self.job.codes.clone();

        eprintln!(
            "[agent {}] starting {} bots, group_size={}, duration={}s",
            self.host_label(),
            codes.len(),
            group_size,
            self.job.duration_secs
        );

        // Spawn bots group by group: leader first (creates the channel), then
        // followers joining its id, so no duplicate same-name channels are made.
        let mut bots: Vec<BotProc> = Vec::with_capacity(codes.len());
        for (group_index, chunk) in codes.chunks(group_size).enumerate() {
            let channel = self.channel_name(group_index);
            let mut channel_id: Option<String> = None;

            for (within, (gamertag, code)) in chunk.iter().enumerate() {
                let bot = BotProc::spawn(
                    &self.bin,
                    gamertag,
                    code,
                    &self.job.server,
                    &channel,
                    channel_id.as_deref(),
                )?;

                if within == 0 {
                    match bot.await_channel(Self::CONNECT_TIMEOUT).await {
                        Some(id) => channel_id = Some(id),
                        None => eprintln!(
                            "[agent {}] leader {} never joined channel {}; followers will create-or-join by name",
                            self.host_label(),
                            gamertag,
                            channel
                        ),
                    }
                }

                bots.push(bot);
                tokio::time::sleep(Self::SPAWN_STAGGER).await;
            }
        }

        // Positions are advertised by the CONTROLLER for the whole simulated
        // realm in one request, because datagram size is driven by
        // players-per-request. An agent posting only its own bots would pin that
        // axis at `bots_per_container` no matter how large the swarm got. The
        // controller starts advertising before any container launches, so the
        // server cache is already populated by the time bots connect.

        // Feed the tone in 1-second batches (50 frames). The client bin paces the
        // 20ms cadence itself, so real-time supply keeps its input buffer shallow.
        let mut tones: Vec<Tone> = (0..bots.len()).map(|_| Tone::new()).collect();
        eprintln!("[agent {}] streaming audio", self.host_label());
        for _second in 0..self.job.duration_secs {
            for (bot, tone) in bots.iter_mut().zip(tones.iter_mut()) {
                if bot.snapshot().disconnected {
                    continue;
                }
                for _ in 0..50 {
                    let frame = tone.next_frame();
                    if bot.feed(frame).await.is_err() {
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Read counters, then tear each bot down cleanly.
        for bot in bots.iter_mut() {
            let _ = bot.request_stats().await;
        }
        tokio::time::sleep(Duration::from_millis(750)).await;

        let mut reports = Vec::with_capacity(bots.len());
        for bot in bots.iter() {
            let s = bot.snapshot();
            reports.push(BotReport {
                name: bot.name().to_string(),
                connected: s.connected,
                disconnected_early: s.disconnected,
                frames_sent: s.frames_sent,
                frames_received: s.frames_received,
            });
        }

        for bot in bots.iter_mut() {
            bot.shutdown().await;
        }

        let report = AgentReport::from_bots(self.host_label(), reports);
        eprintln!(
            "[agent {}] done: {}/{} connected, {} frames received",
            self.host_label(),
            report.connected,
            report.bots.len(),
            report.total_frames_received
        );
        Ok(report)
    }
}
