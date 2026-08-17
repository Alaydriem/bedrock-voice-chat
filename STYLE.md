# Wiki style

Two voices. Pick by directory.

| Directory | Voice | Reader |
|---|---|---|
| `wiki/player/` | Apple Support | Wants to fix one thing and get back to playing |
| `wiki/server/`, `wiki/reference/`, `wiki/creator/`, `wiki/platforms/`, `wiki/upgrading/` | Arch Wiki | Runs the thing, reads a table faster than a paragraph |
| `wiki/start/` | Apple Support | Has not installed anything yet |

## Both voices

The reader came for an answer. Give the answer.

### Name the product, then say what it does for the reader

Open a page or a major section by naming the feature and what it lets the
reader do. The product is the subject, not the reader.

```
BAD   You hear two sets of people: players near you, and anyone in your group.
GOOD  Bedrock Voice Chat lets you hear people nearby you in the world, and in
      groups you create or join.

BAD   BVC plays audio clips in the world.
GOOD  The Jukebox feature of Bedrock Voice Chat enables you to play custom
      audio files in your world and experience it with proximity.
```

Spell out **Bedrock Voice Chat** on first use and inside warnings. `BVC` is
fine after that, and in tables.

An audience gate is allowed as an opening line: "This page is for players who
already have a server running BVC."

### Never explain the documentation

Delete any sentence about how a page was written, what it does or does not
cover, or how it relates to another page. An audience gate is the exception
above.

```
BAD   Generated from source on every build, so it always matches the client.
BAD   What follows is orientation, not a substitute.
BAD   This page does not restate any of it.
GOOD  (nothing — link the reference and move on)
```

### Put caveats in asides

Anything conditional, breakable, or nice-to-know belongs in a Starlight aside
instead of an inline paragraph.

| Aside | Carries |
|---|---|
| `:::caution` | A prerequisite, or something that will break |
| `:::danger` | Data loss, a security problem, or a step that silently defeats BVC |
| `:::tip` | Useful extra the reader can skip |
| `:::note` | Context that qualifies the paragraph above |

`note`, `tip`, `caution` and `danger` are the only valid types. `:::notice`
and `:::warning` render as literal text. Three colons, never two.

Emphasis inside an aside is loud on purpose: `**MUST**`, `**FIRST**`,
`**will not work**`.

`be sure to` and `make sure you` are permitted inside an aside and nowhere
else. `check:prose` enforces exactly that.

### Never restate a generated reference

`bedrockvoicechat.com/api` and `/websocket` are built from source. Link them.
A hand-copied endpoint list is duplication no matter how well written.

### Cut mechanism from player pages, keep it on operator pages

On `wiki/player/`, `wiki/start/`, `wiki/platforms/` and `wiki/creator/`, keep
the happy path. Drop the failure enumerations, the internal mechanism, and the
edge cases, or move them into an aside. Link the operator page instead.

On `wiki/server/` and `wiki/reference/`, keep everything. An operator debugging
at midnight needs the failure mode and the exact default. A reference page that
drops them stops being a reference.

### One "why" per fact, and only if it changes an action

Ask: does knowing this change what the reader types, clicks, or buys? If no,
cut it.

```
GOOD  Set 8444 and 8443. Binding 443 needs root on Linux.
BAD   The proxy relies on migration being permitted.
BAD   Empty frames cost a few bytes twice a second.
```

A surviving "why" gets its own short sentence. Never a trailing clause.

```
BAD   disconnect is idempotent, so a controller that has lost track of the
      client can send it without asking first.
GOOD  disconnect stops the live session. Idempotent; safe to send when
      nothing is connected.
```

### Banned constructions

| Banned | Use instead |
|---|---|
| `X, so Y` / `X, because Y` / `which means` | Two sentences, or cut Y |
| `rather than` as a rhetorical hinge | State what it is. Drop what it isn't |
| "worth knowing", "note that", "importantly", "the thing to know" | Just say the thing |
| Em-dash asides | A period, or parentheses |
| "You are responsible for X" | "Send X." |
| Anthropomorphism (docs that lie, servers that answer "exactly like this") | Describe the behaviour |

### Section titles are nouns or tasks, never claims

Arch-voice pages only. Apple Support voice titles a section with the reader's
problem, which is covered below.

```
BAD   Voice mode decides what mute means
GOOD  Voice mode

BAD   Recording is desktop only, and the server can forbid it
GOOD  Recording availability

BAD   The embedded configuration mirrors the server
GOOD  Embedded configuration
```

Sentence case for a heading naming a thing: "Client application", "Spatial
export". Title Case where the heading is a proper feature name: "How Bedrock
Voice Chat Works", "Getting Started".

### Structure beats prose

Anything with a name, a default, and a description is a table row. Anything
with an order is a numbered list. Prose is what is left over.

## Arch Wiki voice — operator, reference, creator

Declarative and imperative. Assume the reader administers systems.

- Imperative for actions. "Set `transfer_port` to 19139."
- Present indicative for facts. "TLS is required."
- No hedging. Not "you may want to", "it is recommended that", "consider".
- No reassurance. Not "don't worry", "this is straightforward".
- Defaults, ports, paths and commands in tables or code blocks.
- Name the file being edited before the snippet.

```markdown
## Embedded mode

Set `use-embedded-server: true`. `embedded-config` takes the same keys as the
server's `config.hcl`.

| Key | Default | Note |
|---|---|---|
| `server.port` | `443` | Needs root on Linux. Use 8444. |
| `server.quic_port` | `443` | Use 8443. |
| `server.bedrock.transfer_port` | `19132` | Conflicts with Geyser. Use 19139. |

TLS is required. Set `server.tls.certificate` and `server.tls.key`, or
`server.tls.acme`.
```

## Apple Support voice — player, start

Short sentences. One idea each. Numbered steps for anything with an order.

- Title the section as the reader's task or problem. "Test your microphone."
  "Nobody can hear me."
- Name the UI path the way it appears on screen. "Open Settings, then Audio."
- Say what the reader sees after each step.
- Give the fix, then the fallback. Never the mechanism.
- No jargon without the plain word first.

```markdown
## Test your microphone

1. Open Settings, then Audio.
2. Select Test my microphone.
3. Speak for a few seconds.

The mark fills as BVC hears you. If it stays flat, select a different input
device.
```

## Checking

```bash
npm run check:prose
```

Flags the banned constructions above. It is a regex pass, so it catches the
mechanical tics and not the judgement calls. A flagged line inside a fenced
code block or a link URL is ignored.

There is no per-line suppression, deliberately. A rule that fires on correct
prose is a bug in the rule. Fix it or scope it in `scripts/check-prose.mjs`.
Annotations in the source would outlive the rule that caused them.
