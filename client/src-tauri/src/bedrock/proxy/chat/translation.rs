use log::info;

/// Renders Minecraft translation keys into readable English.
///
/// Server announcements, join and leave notices, and every death message arrive as
/// `TextPacketType::Translate` — a key plus parameters, which the game client would localise.
/// The app is not the game client, so it does that here.
///
/// Curated rather than exhaustive. Mojang's catalogue runs to thousands of keys covering
/// achievements, command feedback and UI text, and relaying all of it would fill a chat log
/// with noise. Anything not listed is dropped and logged with its key, so extending this list
/// is a matter of reading one log line rather than guessing at the catalogue.
pub struct MinecraftTranslation;

impl MinecraftTranslation {
    pub fn render(key: &str, params: &[String]) -> Option<String> {
        // Some servers send the key with a leading `%`; the wire is inconsistent about it.
        let key = key.strip_prefix('%').unwrap_or(key);

        if let Some(template) = Self::template(key) {
            return Some(Self::substitute(template, params));
        }

        // Mojang keeps adding deaths, so the table will always trail the game. The parameter
        // count still carries the important part: a second parameter is the killer, and
        // naming them is the difference between a useful line and a shrug.
        if key.starts_with("death.") {
            let template = if params.len() >= 2 {
                "%1$s was killed by %2$s"
            } else {
                "%1$s died"
            };
            // Named at info because the line the player sees is now a paraphrase. Deaths are
            // infrequent, so this costs nothing and is the only way to learn the exact key
            // worth adding above — the generic text alone gives no clue which one fired.
            info!("Bedrock chat: death key '{key}' has no template; rendered generically");
            return Some(Self::substitute(template, params));
        }

        None
    }

    fn template(key: &str) -> Option<&'static str> {
        Some(match key {
            // `/say`, and the shape most server broadcasts arrive in.
            "chat.type.announcement" => "[%1$s] %2$s",
            "chat.type.text" => "<%1$s> %2$s",
            "chat.type.emote" => "* %1$s %2$s",

            "multiplayer.player.joined" => "%1$s joined the game",
            "multiplayer.player.joined.renamed" => "%1$s (formerly %2$s) joined the game",
            "multiplayer.player.left" => "%1$s left the game",

            "death.attack.generic" => "%1$s died",
            "death.attack.mob" => "%1$s was slain by %2$s",
            "death.attack.player" => "%1$s was slain by %2$s",
            "death.attack.arrow" => "%1$s was shot by %2$s",
            "death.attack.arrow.item" => "%1$s was shot by %2$s using %3$s",
            "death.attack.player.item" => "%1$s was slain by %2$s using %3$s",
            "death.attack.trident" => "%1$s was impaled by %2$s",
            "death.attack.witherSkull" => "%1$s was shot by a skull from %2$s",
            "death.attack.sting" => "%1$s was stung to death",
            "death.attack.fireworks" => "%1$s went off with a bang",
            "death.attack.flyIntoWall" => "%1$s experienced kinetic energy",
            "death.attack.magma" => "%1$s discovered the floor was lava",
            "death.attack.even_more_magic" => "%1$s was killed by even more magic",
            "death.attack.stalactite" => "%1$s was skewered by a falling stalactite",
            "death.attack.stalagmite" => "%1$s was impaled on a stalagmite",
            "death.attack.thorns" => "%1$s was killed trying to hurt %2$s",
            "death.attack.explosion" => "%1$s blew up",
            "death.attack.explosion.player" => "%1$s was blown up by %2$s",
            "death.attack.magic" => "%1$s was killed by magic",
            "death.attack.indirectMagic" => "%1$s was killed by %2$s using magic",
            "death.attack.wither" => "%1$s withered away",
            "death.attack.anvil" => "%1$s was squashed by a falling anvil",
            "death.attack.fallingBlock" => "%1$s was squashed by a falling block",
            "death.attack.lava" => "%1$s tried to swim in lava",
            "death.attack.inFire" => "%1$s went up in flames",
            "death.attack.onFire" => "%1$s burned to death",
            "death.attack.fireball" => "%1$s was fireballed by %2$s",
            "death.attack.drown" => "%1$s drowned",
            "death.attack.starve" => "%1$s starved to death",
            "death.attack.cactus" => "%1$s was pricked to death",
            "death.attack.fall" => "%1$s fell from a high place",
            "death.attack.outOfWorld" => "%1$s fell out of the world",
            "death.attack.lightningBolt" => "%1$s was struck by lightning",
            "death.attack.freeze" => "%1$s froze to death",
            "death.attack.sweetBerryBush" => "%1$s was poked to death by a sweet berry bush",
            "death.fell.accident.generic" => "%1$s fell from a high place",
            "death.fell.accident.ladder" => "%1$s fell off a ladder",
            "death.fell.accident.vines" => "%1$s fell off some vines",
            "death.fell.accident.water" => "%1$s fell out of the water",

            _ => return None,
        })
    }

    /// Handles both `%s` in order and the positional `%1$s` form, which Mojang mixes freely.
    fn substitute(template: &str, params: &[String]) -> String {
        let mut out = String::with_capacity(template.len() + 16);
        let mut chars = template.chars().peekable();
        let mut next_positional = 0usize;

        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }

            match chars.peek() {
                Some('s') => {
                    chars.next();
                    out.push_str(Self::param(params, next_positional));
                    next_positional += 1;
                }
                Some(d) if d.is_ascii_digit() => {
                    let mut index = 0usize;
                    while let Some(d) = chars.peek() {
                        if let Some(v) = d.to_digit(10) {
                            index = index * 10 + v as usize;
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // Consume the `$s` tail of `%1$s`.
                    if chars.peek() == Some(&'$') {
                        chars.next();
                    }
                    if chars.peek() == Some(&'s') {
                        chars.next();
                    }
                    out.push_str(Self::param(params, index.saturating_sub(1)));
                }
                // A literal percent, or something unrecognised. Keep it rather than eat it.
                _ => out.push('%'),
            }
        }

        out
    }

    fn param(params: &[String], index: usize) -> &str {
        params.get(index).map(String::as_str).unwrap_or("")
    }
}
