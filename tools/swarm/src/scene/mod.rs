mod channel_names;
mod chat_beat;
mod chat_script;
mod chat_wire;
mod chatter;
mod config;
mod deck;
mod layout;
mod placement;
mod presence_feed;
mod scenario;
mod troupe;

pub use channel_names::ChannelNames;
pub use chat_beat::ChatBeat;
pub use chat_script::ChatScript;
pub use chat_wire::ChatWireFrame;
pub use chatter::Chatter;
pub use config::SceneConfig;
pub use deck::Deck;
pub use layout::SceneLayout;
pub use placement::Placement;
pub use presence_feed::PresenceFeed;
pub use scenario::Scenario;
pub use troupe::Troupe;

use std::sync::Arc;

use crate::minter::CodeMinter;

/// Composes one scene against a running server and holds it there.
///
/// Holds rather than runs: a screenshot is taken by a person, so the tool's job ends
/// with the picture standing up and staying up until interrupted. Nothing here has a
/// duration.
pub struct SceneDirector {
    config: SceneConfig,
    scenario: Scenario,
    speaking: Vec<String>,
    chat: bool,
    chat_preview: Option<usize>,
}

impl SceneDirector {
    // Long enough that a scene survives a slow framing session, short enough that a
    // leaked code is not interesting. Codes are ephemeral and single-use regardless.
    const CODE_TTL_SECS: u64 = 3_600;

    // Fixed, so a scene's conversation is the same on every run and a retaken screenshot
    // differs only in what actually changed.
    const CHAT_SEED: u64 = 0x5EED_C0DE_1234_5678;

    pub fn new(
        config: SceneConfig,
        scenario: Scenario,
        speaking: Vec<String>,
        chat: bool,
        chat_preview: Option<usize>,
    ) -> Self {
        Self {
            config,
            scenario,
            speaking,
            chat,
            chat_preview,
        }
    }

    /// Print the conversation this scene would carry, touching nothing.
    fn preview(&self, layout: &SceneLayout, lines: usize) {
        let mut cast: Vec<String> = layout.staged.iter().map(|p| p.name.clone()).collect();
        cast.extend(layout.connected.iter().cloned());

        let mut script = ChatScript::new(cast, Self::CHAT_SEED);
        for _ in 0..lines {
            match script.next() {
                Some(ChatBeat::Chat { author, text }) => println!("  <{}> {}", author, text),
                Some(ChatBeat::Event { text }) => println!("  * {}", text),
                None => break,
            }
        }
    }

    pub async fn run(&self) -> Result<(), anyhow::Error> {
        let layout = self.scenario.resolve(&self.config)?;

        if let Some(lines) = self.chat_preview {
            self.preview(&layout, lines);
            return Ok(());
        }

        let speaking = self.resolve_speaking(&layout)?;
        let needs_clients = layout.needs_clients() || !speaking.is_empty();

        // Checked before anything is advertised. Discovering a missing client binary
        // after the roster is live leaves staged players on the server for the next
        // 15 seconds and reports a config mistake as though it were a runtime one.
        if needs_clients {
            self.config.client_identity()?;
        }

        let ca_pem = self.config.ca_pem()?;
        let feed = Arc::new(PresenceFeed::new(
            &self.config,
            &layout.staged,
            ca_pem.as_deref(),
        )?);

        // Positions first, and awaited: the roster is in place before a client connects,
        // which is the order the real system runs in — a mod is advertising long before
        // anyone opens the app.
        let advertised = feed.advertised();
        let feed_task = feed.hold().await?;
        println!(
            "staged {} players around {} at {} Hz",
            advertised - 1,
            self.config.observer,
            self.config.position_hz
        );

        let troupe_task = if needs_clients {
            Some(self.connect(&layout, &speaking).await?)
        } else {
            None
        };

        let chat_task = if self.chat {
            Some(self.start_chat(&layout).await?)
        } else {
            None
        };

        if let Some(group) = &layout.group_name {
            println!(
                "group \"{}\" is live — join it by that name in the app, or it will not appear in your rail",
                group
            );
        }

        println!("holding the scene; Ctrl+C to tear it down");
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| anyhow::anyhow!("waiting for interrupt: {}", e))?;

        feed_task.abort();
        if let Some(task) = troupe_task {
            task.abort();
        }
        if let Some(task) = chat_task {
            task.abort();
        }

        // Staged players leave on their own: the server holds a player for 15 seconds
        // after their last report, so stopping the feed is the teardown.
        println!("torn down; staged players expire from the server within 15s");
        Ok(())
    }

    async fn connect(
        &self,
        layout: &SceneLayout,
        speaking: &[String],
    ) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
        let (admin_cert, admin_key, client_bin) = self.config.client_identity()?;
        let ca_path = self.config.ca_path();
        let minter = CodeMinter::from_paths(
            self.config.server_base(),
            ca_path.as_deref().and_then(|p| p.to_str()),
            admin_cert.to_str().ok_or_else(|| {
                anyhow::anyhow!("admin_cert path is not valid unicode: {}", admin_cert.display())
            })?,
            admin_key.to_str().ok_or_else(|| {
                anyhow::anyhow!("admin_key path is not valid unicode: {}", admin_key.display())
            })?,
        )?;

        let bin = client_bin
            .to_str()
            .ok_or_else(|| {
                anyhow::anyhow!("client_bin path is not valid unicode: {}", client_bin.display())
            })?
            .to_string();

        let mut troupe = Troupe::new(bin, self.config.server_base().to_string());

        if let (Some(group), false) = (&layout.group_name, layout.connected.is_empty()) {
            troupe
                .join_group(&minter, &layout.connected, group, Self::CODE_TTL_SECS)
                .await?;
            println!(
                "connected {} members into \"{}\"",
                layout.connected.len(),
                group
            );
        }

        if !speaking.is_empty() {
            troupe.add_speakers(&minter, speaking, Self::CODE_TTL_SECS).await?;
            println!("transmitting: {}", speaking.join(", "));
        }

        println!("{} clients connected", troupe.connected());
        Ok(troupe.hold())
    }

    /// Start the chat feed, voiced by the people the scene actually placed.
    ///
    /// Group members are included even though they are not on the ring: they are on the
    /// server, and a group whose members never say anything in chat reads as a roster rather
    /// than as a room.
    async fn start_chat(
        &self,
        layout: &SceneLayout,
    ) -> Result<tokio::task::JoinHandle<()>, anyhow::Error> {
        let mut cast: Vec<String> = layout.staged.iter().map(|p| p.name.clone()).collect();
        cast.extend(layout.connected.iter().cloned());

        let period = std::time::Duration::from_millis(self.config.chat_period_ms.max(1));
        let chatter = Chatter::new(&self.config, period)?;
        let task = chatter
            .hold(ChatScript::new(cast.clone(), Self::CHAT_SEED))
            .await?;

        println!(
            "chat is live in \"{}\" — {} voices, a line every {} ms",
            self.config.world_name,
            cast.len(),
            self.config.chat_period_ms
        );
        Ok(task)
    }

    // A speaker has to be somebody already on the ring. Connecting a name that is not
    // staged produces a player with a voice connection and no position, which the feed
    // reports to nobody — audible, invisible, and indistinguishable from a broken scene.
    fn resolve_speaking(&self, layout: &SceneLayout) -> Result<Vec<String>, anyhow::Error> {
        let unstaged: Vec<&str> = self
            .speaking
            .iter()
            .filter(|name| !layout.staged.iter().any(|p| &&p.name == name))
            .map(|name| name.as_str())
            .collect();

        if !unstaged.is_empty() {
            return Err(anyhow::anyhow!(
                "--speaking names {} which this scenario does not stage; it places {}",
                unstaged.join(", "),
                layout
                    .staged
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        Ok(self.speaking.clone())
    }
}
