![YouTube Channel Subscribers](https://img.shields.io/youtube/channel/subscribers/UCXgqRZv7bHsKzwYBrtA9DFA?label=Youtube%20Subscribers&logo=Alaydriem&style=flat-square)

<div align="center">

  <h1>Bedrock Voice Chat</h1>

<a href="https://www.youtube.com/@Alaydriem"><img src="https://raw.githubusercontent.com/alaydriem/bedrock-material-list/master/docs/subscribe.png" width="140"/></a>

<a href="https://discord.gg/CdtchD5zxr"><img src="https://raw.githubusercontent.com/alaydriem/bedrock-voice-chat/master/pack/pack/bp/pack_icon.png" width="140"/></a>

  <p>
    <strong>A High Performance, Low Latency, Secure Voice Chat server for Minecraft Bedrock Dedicated Servers</strong>
  </p>
  <hr />
</div>

Bedrock Voice Chat is an experimental addon for Minecraft Bedrock edition that provides a high performance, low latency and secure voice chat server written in Rust for Minecraft Bedrock Dedicated Server, with the objective of having feature parity with Simple Voice Chat for Java, and a significantly improved experience for Bedrock users.

Features include (most are WIP).

- Support for WSAPI + ASIO inputs
- Proximity Chat with Positional Audio
- Client side voice deafening/muting
- Client Audio controls via Windows 10 _and_ Android + iOS Native applications (muting, deafening, individual volume slider)
- Configurable Push-to-talk/Voice Activation
- Password Protected Group Chats
- Audio Recording to separate audio tracks for later download/processing
- Noise Suppression, Audio Amplification
- TLS with message + package encryption via libsodium // ncryptf (X25519 + XSalsa20-Poly1305).
- And More

## Installation

Bedrock Voice Chat (BVC) has serveral components that need to be installed for it to work for everyone involved.

### Bedrock Dedicated Server

BVC ships with a Bedrock Dedicated Server (BDS) Behavior pack that needs to be installed to your BDS server to relay client positioning data. Your BDS server requires a few additional configurations beyond the normal plugin installation process.

1. Update `config/default/permissions.json` with at minimum, the following to enable BDS server to run the module:

```json
{
  "allowed_modules": [
    "@minecraft/server-gametest",
    "@minecraft/server",
    "@minecraft/server-ui",
    "@minecraft/server-net",
    "@minecraft/server-admin",
    "@minecraft/server-editor"
  ]
}
```

2. In `config/default/variable.json`, update the `bvc_server` property to be the full fully-qualified domain name + scheme of your BVC server port configured in the next step:

> Reminder: these are _examples_. Don't blindly copy/paste them into your configuration and BVC won't work with them.

For instance, if you're running BVC on the same server as your use (PiglinHost, Apex, Docker, etc...), you might set this to:

```json
{
  "bvc_server": "http://127.0.0.1:3000"
}
```

If you're running BVC Server on a separate host, your config may look like:

```json
{
  "bvc_server": "https://heart-and-soul.bvc.alaydriem.com"
}
```

### BVC Server

### BVC Windows 10 Client

### BVC Android/iOS Client

[{"name":"Alaydriem","dimension":"minecraft:overworld","coordinates":{"x":0.5,"y":70,"z":0.5},"deafen":false}]
