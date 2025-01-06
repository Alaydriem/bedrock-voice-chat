![YouTube Channel Subscribers](https://img.shields.io/youtube/channel/subscribers/UCXgqRZv7bHsKzwYBrtA9DFA?label=Youtube%20Subscribers&logo=Alaydriem&style=flat-square)

<div align="center">

  <h1>Bedrock Voice Chat Client</h1>

<a href="https://www.youtube.com/@Alaydriem"><img src="https://raw.githubusercontent.com/alaydriem/bedrock-material-list/master/docs/subscribe.png" width="140"/></a>

<a href="https://discord.gg/CdtchD5zxr"><img src="https://raw.githubusercontent.com/alaydriem/bedrock-voice-chat/master/pack/pack/bp/pack_icon.png" width="140"/></a>

  <p>
    <strong>A client application for Bedrock Voice Chat, written in Rust, Svelte, Typescript - and Tauri.</strong>
  </p>
  <hr />
</div>

Bedrock Voice Chat is an experimental addon for Minecraft Bedrock edition that provides a high performance, low latency and secure voice chat server written in Rust for Minecraft Bedrock Dedicated Server, with the objective of having feature parity with Simple Voice Chat for Java, and a significantly improved experience for Bedrock users.

This is the default client that ships with Bedrock Voice Chat with the following planned features:

[ ] Cross Platform Builds with Tauri for

- [ ] Windows
- [ ] Android
- [ ] iOS (iPhone and iPad)
- [ ] MacOS (Experimental)
- [ ] Linux (Experimental)
      [ ] ASIO on Windows
      [ ] Dedicated Group Chat / Support
      [ ] Shared Voice Chat
      [ ] Noise Gate Control
      [ ] AI Background Noise Filtering

And more

## Usage Requirements

BVC Client is still in early development, and therefore some features require client setup external to your device

### Windows

- Audio input and output devices must be configured for 48khz for opus frame packets to be heard and sent correctly. A future enhancement will allow for suboptimal translation.

### iOS

### Android

## Development Requirements

- NodeJS
- Yarn
- Rust Stable
- Zig + Cargo Zigbuild (cargo install zigbuild)
- Cargo cross
- Android Studio
- XCode
- Visual Studio 202x

## Building

The Tauri app natively supports cross-compilation for all supported targets, however builds should be execute on Windows and cross-built to other platforms using remote deployment tools via Android Studio and XCode.

### Desktop Building

```
yarn tauri dev
```

### Android Building

Android builds require libsodium-sys, and cross compilation of libsodium-sys for Android architectures, which in turn depends on a Unix-like system to build it.

```
yarn tauri android dev
```

### iOS Building

```
yarn tauri ios dev
```
