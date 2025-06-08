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

Additionally, ensure that `LIBCLANG_PATH` is defined in your environment variables. Default starting values are:

```
# Windows 10 VS 2022 Community Edition
LIBCLANG_PATH = { value = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\Llvm\\x64\\lib", relative = false }

# MacOS XCode ToolChain
#LIBCLANG_PATH = { value = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib", relative = false }

```

## Building

The Tauri app natively supports cross-compilation for all supported targets, however builds should be execute on Windows and cross-built to other platforms using remote deployment tools via Android Studio and XCode.

### Desktop Building

```
yarn tauri dev
```

### Android Building

Android development has only been tested on a Windows Machine, though it is likely to work elsewhere assuming all the dependencies are met:

For Windows:
1. Install vcpkg, and install openss and libsodium for Android targets
2. Set your environment variables as follows:

```
# Set paths
# Set paths
$env:OPENSSL_DIR="$env:USERPROFILE\projects\vcpkg\installed\arm64-android"
$env:SODIUM_LIB_DIR="$env:USERPROFILE\projects\vcpkg\installed\arm64-android\lib"
$env:SODIUM_INCLUDE_DIR="$env:USERPROFILE\projects\vcpkg\installed\arm64-android\include"
$env:SODIUM_SHARED=0

# Set General Compiler Options
$env:CMAKE_GENERATOR="Ninja"
$env:CMAKE="cmake"
$env:TARGET="aarch64-linux-android"

# AWS LC SYS specific settings
$env:AWS_LC_SYS_CMAKE_GENERATOR="Ninja"
$env:AWS_LC_SYS_CMAKE_BUILDER=1
$env:AWS_LC_SYS_EXTERNAL_BINDGEN=0
$env:BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot"
$env:RUSTFLAGS="-C link-arg=-lc++_shared"
```

4. You can then try to build for Android by running. Make sure you have a valid target device, in this case an arm64
```
yarn tauri android dev --open
```

This will build, compile, and install onto the device, however the app currently crashes when the Activity is launched.




### iOS Building

```
yarn tauri ios dev
```

> You'll need to cross-compile and build this with a native XCode build. Additionally, you'll need a fully validate code signing certificate and a full apple developer account (paid) for entitlement support.
