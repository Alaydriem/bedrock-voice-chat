use std::time::Duration;

use crate::bot_proc::BotProc;
use crate::minter::CodeMinter;
use crate::scene::ChannelNames;
use crate::tone::Tone;

/// The players a scene has to connect for real.
///
/// Staging a position is enough to be seen on the ring, and it is not enough for two
/// things: group membership, which the server writes only for the authenticated client
/// asking for itself, and speaking, which needs audio actually arriving. Both cost a
/// process per player and an admin identity to mint them, which is why a scenario that
/// needs neither never touches this.
pub struct Troupe {
    bin: String,
    server: String,
    bots: Vec<BotProc>,
    speaking: Vec<usize>,
}

impl Troupe {
    // A leader has to create the channel and report its id before followers can join it
    // rather than racing to create their own.
    const CHANNEL_TIMEOUT: Duration = Duration::from_secs(20);

    // Enough for the connect to settle before the next one starts.
    const SPAWN_STAGGER: Duration = Duration::from_millis(400);

    // The client paces the 20 ms cadence itself, so a second of frames at a time keeps
    // its input buffer shallow without this loop having to be precise.
    const FRAMES_PER_SECOND: usize = 50;

    pub fn new(bin: String, server: String) -> Self {
        Self {
            bin,
            server,
            bots: Vec::new(),
            speaking: Vec::new(),
        }
    }

    /// Connect `names` into one channel, so the group is real on the server.
    ///
    /// The first member speaks. A group panel where nobody has ever transmitted shows
    /// membership and no activity, which is a truthful picture of an idle group and a
    /// poor picture of the feature.
    pub async fn join_group(
        &mut self,
        minter: &CodeMinter,
        names: &[String],
        channel: &str,
        code_ttl_secs: u64,
    ) -> Result<(), anyhow::Error> {
        let codes = minter.mint(names, code_ttl_secs).await?;
        let mut channel_id: Option<String> = None;

        for (index, (gamertag, code)) in codes.iter().enumerate() {
            let bot = BotProc::spawn(
                &self.bin,
                gamertag,
                code,
                &self.server,
                channel,
                channel_id.as_deref(),
            )?;

            if index == 0 {
                match bot.await_channel(Self::CHANNEL_TIMEOUT).await {
                    Some(id) => channel_id = Some(id),
                    None => {
                        return Err(anyhow::anyhow!(
                            "{} never joined {}; without the channel id the others would each create their own",
                            gamertag,
                            channel
                        ));
                    }
                }
                self.speaking.push(self.bots.len());
            }

            self.bots.push(bot);
            tokio::time::sleep(Self::SPAWN_STAGGER).await;
        }

        Ok(())
    }

    /// Connect staged players for real so they transmit, all of them at once.
    ///
    /// Each gets a channel of its own, which nobody else joins. One shared channel would
    /// make them same-channel and route their audio non-spatially — the opposite of what
    /// a proximity scene is showing — and no channel at all is not something the client
    /// harness offers.
    pub async fn add_speakers(
        &mut self,
        minter: &CodeMinter,
        names: &[String],
        code_ttl_secs: u64,
    ) -> Result<(), anyhow::Error> {
        let codes = minter.mint(names, code_ttl_secs).await?;

        for (gamertag, code) in codes.iter() {
            // Numbered from the speakers already connected rather than from this call, so
            // a second call cannot hand out a channel name the first one is using.
            let bot = BotProc::spawn(
                &self.bin,
                gamertag,
                code,
                &self.server,
                &ChannelNames::solo(self.speaking.len()),
                None,
            )?;

            self.speaking.push(self.bots.len());
            self.bots.push(bot);
            tokio::time::sleep(Self::SPAWN_STAGGER).await;
        }

        Ok(())
    }

    pub fn connected(&self) -> usize {
        self.bots.len()
    }

    /// Stream a tone from every speaking member until the task is aborted.
    ///
    /// Each bot keeps its own phase so the tones do not sum into one louder tone, which
    /// is what makes a level meter read as several people rather than one loud one.
    pub fn hold(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut tones: Vec<Tone> = self.speaking.iter().map(|_| Tone::new()).collect();

            loop {
                for (slot, index) in self.speaking.iter().enumerate() {
                    let bot = &mut self.bots[*index];
                    if bot.snapshot().disconnected {
                        continue;
                    }
                    for _ in 0..Self::FRAMES_PER_SECOND {
                        let frame = tones[slot].next_frame();
                        if bot.feed(frame).await.is_err() {
                            break;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    }
}
