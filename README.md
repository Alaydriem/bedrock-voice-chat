

<div align="center">

  <h1>Bedrock Voice Chat</h1>

<a href="https://www.youtube.com/@Alaydriem"><img src="https://img.shields.io/youtube/channel/subscribers/UCXgqRZv7bHsKzwYBrtA9DFA?label=Youtube%20Subscribers&logo=Alaydriem&style=flat-square" width="140"/></a>

<a href="https://discord.gg/MAHckcEATj"><img src="https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/master/client/static/images/app-logo-transparent.png" width="140"/></a>

  <p>
    <strong>A High Performance, Low Latency, Secure Voice Chat server for Minecraft Bedrock Dedicated Servers</strong><br /><br />
    <a href="https://www.bedrockvoicechat.com">www.bedrockvoicechat.com</a>
  </p>
  <hr />
</div>

Bedrock Voice Chat is an client app + addon for Minecraft Bedrock edition that provides a high performance, low latency and secure voice chat server written in Rust for Minecraft Bedrock Dedicated Server, with the objective of having feature parity with Simple Voice Chat for Java, and a significantly improved experience for Bedrock users.

- Support for WSAPI + ASIO inputs
- Proximity Chat with Positional Audio
- Group Chats for persistent audio
- Works on Desktop (Windows 10/11), iOS, and Android
- Client Audio Controls (muting, deafening, individual volume slider)
- Audio Recording to separate audio tracks for later download/processing with Timecode support
- Noise Suppression, Audio Amplification
- Voice Detection
- Fully Encrypted
- Opus Audio Codec
- And More

## Getting Started

[www.bedrockvoicechat.com](https://www.bedrockvoicechat.com) is your source for getting started, finding the most recent download, and getting information on how to run and manage Bedrock Voice Chat. Be sure to checkout the wiki at: https://www.bedrockvoicechat.com/wiki for comprehensive documentation.

For operators running cross-server voice relay (Realms Connect / Proxy Connect with multiple BVC servers sharing a world), see [docs/relay-performance.md](docs/relay-performance.md) for bandwidth and scaling guidance.

## How you can Support

I’ve set up a few ways for you to join the supporters who help keep Bedrock Voice Chat thriving - checkout https://github.com/Alaydriem/bedrock-voice-chat/discussions/31 for more details.

1. Become a Channel Member or Twitch Subscriber
Join the community on [YouTube](https://youtube.com/@alaydriem)
 or [Twitch](https://twitch.tv/alaydriem)
 — members get exclusive updates, early feature previews, and a front-row seat to development progress.

2. Back Me on Patreon
Visit [Patreon.com/Alaydriem](https://www.patreon.com/c/Alaydriem) and become a Bedrock Voice Chat Supporter. Every pledge directly funds ongoing development, app store maintenance, and future innovations.

Supporters and Sponsors like you make projects like Bedrock Voice Chat possible, and enable us to fulfill feature requests and publications to the Google Play and Apple App Stores. Supporter and Sponsors at these roles will receive these updates before they are unlocked for everyone.

## Installation

Bedrock Voice Chat has 3 components

1. A BDS behavior pack (or Java mod) you install on your server that connects to -
2. A Bedrock Voice Chat Server you need to host and maintain that ties position data and audio data together
3. And a native client for Windows, iOS, and Android.

All 3 components are mandatory. Be sure to checkout the Wiki for more information: https://www.bedrockvoicechat.com/wiki

## Building from Source

Builds and tests run through [mise](https://mise.jdx.dev/). Every build, test and maintenance
command in this repository is a mise task that sets the working directory, the toolchain and the
mandatory flags for you. A bare `cargo build` or `yarn tauri build` misses at least one of those
and fails with an error that does not name its cause.

### Prerequisites

Install these yourself:

| Requirement | Notes |
| --- | --- |
| [mise](https://mise.jdx.dev/getting-started.html) | Supplies node, yarn and the two Java versions. Everything below assumes it is on `PATH` |
| Rust | Pinned by `rust-toolchain.toml`; `rustup` installs the pinned version automatically |
| `cargo-nextest` | `cargo install cargo-nextest --locked`. Client tests require it — `cargo test` is not a substitute |
| Visual Studio 2022 Build Tools with Clang (Windows) | Set `LIBCLANG_PATH`, for example `C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\Llvm\x64\lib` |

mise provides node, yarn, Java 21 for the Android client build, and Java 25 for the Java mods.
You do not install those separately.

### First build

```bash
git clone https://github.com/Alaydriem/bedrock-voice-chat
cd bedrock-voice-chat

mise trust && mise trust client/mise.toml && mise trust mods/java/mise.toml
mise run setup
mise run preflight
```

`mise trust` is required once per checkout, per config file — mise refuses to read a config it has
not been told to trust. `mise run setup` downloads the Steinberg ASIO SDK, installs the client JS
dependencies, builds the i18n resources and builds the SvelteKit frontend; it takes about
15 seconds. `mise run preflight` then reports anything still missing by name, rather than letting
a build fail on it later.

### Tasks

Run `mise run <task>` from anywhere in the checkout. `mise tasks` lists all of them with
descriptions.

| Task | What it does |
| --- | --- |
| `mise run build-client` | Builds the Tauri desktop client |
| `mise run build-server` | Builds the voice chat server |
| `mise run check-common` | Type-checks the shared `common` crate |
| `mise run test-client [filter]` | Runs the client test suite, building the server cdylib and e2e binary first |
| `mise run test-server` | Runs the server test suite |
| `mise run test-common` | Runs the shared crate tests |
| `mise run bindings` | Regenerates the ts-rs TypeScript bindings in `client/src/js/bindings/`, and the barrel that re-exports them |
| `mise run frontend` | Builds the SvelteKit frontend into `client/build` |
| `mise run dev-bds` | Builds and deploys the BDS packs, then starts a local Bedrock Dedicated Server |
| `mise run dev-paper` | Builds and deploys the Paper plugin, then starts a local Paper server |
| `mise run dev-fabric` | Builds and deploys the Fabric mod, then starts a local Fabric server |
| `mise run preflight` | Reports environment problems by name |
| `mise run targets-report` | Reports the size and age of the cargo target directories |
| `mise run clean-incremental` | Reclaims cargo incremental caches. Dry-run unless you pass `--force` |
| `mise run clean-targets [days]` | Deletes target directories idle for N days. Dry-run unless you pass `--force` |

Tasks declare their own dependencies, so asking for the end product is enough: `test-client`
builds what it needs first, and `frontend` installs dependencies and builds the i18n catalogs
first. Flags for the underlying tool go after a `--` separator, so mise does not read them as
its own: `mise run dev-bds -- --no-net`.

Rust builds in this repository are large. `mise run targets-report` shows what the cargo target
directories currently cost, and `clean-incremental` reclaims the incremental caches — pure build
state, at the price of one non-incremental rebuild.

### Minecraft mods

Each platform has a task that builds the mod, deploys it to your local test server and then
starts that server attached to your terminal. Ctrl+C stops it; re-run to rebuild and restart.

```bash
mise run dev-bds      # Bedrock Dedicated Server
mise run dev-paper    # Paper, with the Simple Voice Chat bridge
mise run dev-fabric   # Fabric, with the Simple Voice Chat bridge
```

Server paths come from `mods/.env`; copy `mods/.env.example` to get started. The tasks install
the yarn dependencies and, for Paper and Fabric, stage the relay SDK native library first — that
library is not committed and the bridge fails without it. Pass `-- --release`, `-- --no-build`, or
(BDS only) `-- --no-net` to change what they do.

`mise run svc-paper` still works as an alias for `dev-paper`. `mise run svc-native` stages the
relay native on its own, and `mise run svc-verify` prints the manual checklist for testing the
bridge by hand.

To build the mods without starting a server, `mods/` also exposes `yarn dev-build` (native
library and Paper) and `yarn dev-build-all` (adds Fabric and the BDS packs). Run these from
`mods/java/` or wrap them in `mise x java@temurin-25 --`, so gradlew gets the pinned JDK instead
of whatever java is first on `PATH`.

## Telemetry & Metrics

The Bedrock Voice Chat server exposes operational metrics and, optionally, sends anonymous aggregate usage data.

- **Local metrics (always on):**
  - **Prometheus** — scrape `GET /metrics` (served on the same port as the HTTP API).
  - **statsd / dogstatsd** — pushed over UDP to `127.0.0.1:8125` (fail-open; if no agent is listening, packets are simply dropped).
  - Metrics include connected players, active channels, players in channels, and connect/disconnect/channel-join/leave counts. No player identity is ever included.

- **Fleet usage telemetry (opt-out):** when `server.features.telemetry = true` (the default), the server sends **anonymous, aggregate** events — a one-time `server_started` ping plus player connect/disconnect and channel join/leave events — to the project's PostHog. These carry **no player identity** (gamertag, UUID, or name); events are grouped only by an anonymous per-deployment id derived from your CA certificate. Set `telemetry = false` in your config's `features {}` block to disable all outbound telemetry (local `/metrics` and statsd remain available).


