#!/bin/bash

# Cross-platform Android build script for GitHub Actions
# Supports both Linux and Windows (via Git Bash/MSYS2)

set -e

BUILD_TYPE="${1:-debug}"

echo "Starting Android build in $BUILD_TYPE mode..."

# Detect platform
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    PLATFORM="windows"
    NDK_PREBUILT_DIR="windows-x86_64"
    CLANG_SUFFIX=".cmd"
    AR_SUFFIX=".exe"
else
    PLATFORM="linux"
    NDK_PREBUILT_DIR="linux-x86_64"
    CLANG_SUFFIX=""
    AR_SUFFIX=""
fi

echo "Detected platform: $PLATFORM"

# Validate required environment variables
if [[ -z "$ANDROID_NDK_HOME" ]]; then
    echo "Error: ANDROID_NDK_HOME is not set"
    exit 1
fi

if [[ ! -d "$ANDROID_NDK_HOME" ]]; then
    echo "Error: Android NDK not found at $ANDROID_NDK_HOME"
    exit 1
fi

# Set environment variables for build tools
export CMAKE_GENERATOR="Ninja"
export CMAKE="cmake"
export AWS_LC_SYS_CMAKE_GENERATOR="Ninja"
export AWS_LC_SYS_CMAKE_BUILDER="1"
export AWS_LC_SYS_EXTERNAL_BINDGEN="1"

# Add NDK tools to PATH
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/bin:$PATH"

# Force all architectures to use API 27 toolchain for AAudio support
export CC_aarch64_linux_android="aarch64-linux-android27-clang${CLANG_SUFFIX}"
export CXX_aarch64_linux_android="aarch64-linux-android27-clang++${CLANG_SUFFIX}"
export AR_aarch64_linux_android="llvm-ar${AR_SUFFIX}"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="aarch64-linux-android27-clang${CLANG_SUFFIX}"

export CC_armv7_linux_androideabi="armv7a-linux-androideabi27-clang${CLANG_SUFFIX}"
export CXX_armv7_linux_androideabi="armv7a-linux-androideabi27-clang++${CLANG_SUFFIX}"
export AR_armv7_linux_androideabi="llvm-ar${AR_SUFFIX}"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="armv7a-linux-androideabi27-clang${CLANG_SUFFIX}"

export CC_i686_linux_android="i686-linux-android27-clang${CLANG_SUFFIX}"
export CXX_i686_linux_android="i686-linux-android27-clang++${CLANG_SUFFIX}"
export AR_i686_linux_android="llvm-ar${AR_SUFFIX}"
export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="i686-linux-android27-clang${CLANG_SUFFIX}"

export CC_x86_64_linux_android="x86_64-linux-android27-clang${CLANG_SUFFIX}"
export CXX_x86_64_linux_android="x86_64-linux-android27-clang++${CLANG_SUFFIX}"
export AR_x86_64_linux_android="llvm-ar${AR_SUFFIX}"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="x86_64-linux-android27-clang${CLANG_SUFFIX}"

# Set RUSTFLAGS for each target with library search paths
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/lib/aarch64-linux-android/27"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RUSTFLAGS="--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/lib/arm-linux-androideabi/27"
export CARGO_TARGET_I686_LINUX_ANDROID_RUSTFLAGS="--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/lib/i686-linux-android/27"
export CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/lib/x86_64-linux-android/27"

# Set base includes (clang builtin headers and general sysroot)
if [[ "$PLATFORM" == "windows" ]]; then
    INCLUDES_BASE="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/lib/clang/20/include;$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include"
else
    INCLUDES_BASE="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/lib/clang/20/include:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include"
fi

# Set target-specific includes for each architecture
if [[ "$PLATFORM" == "windows" ]]; then
    export AWS_LC_SYS_INCLUDES_aarch64_linux_android="$INCLUDES_BASE;$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/aarch64-linux-android"
    export AWS_LC_SYS_INCLUDES_armv7_linux_androideabi="$INCLUDES_BASE;$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/arm-linux-androideabi"
    export AWS_LC_SYS_INCLUDES_i686_linux_android="$INCLUDES_BASE;$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/i686-linux-android"
    export AWS_LC_SYS_INCLUDES_x86_64_linux_android="$INCLUDES_BASE;$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/x86_64-linux-android"
else
    export AWS_LC_SYS_INCLUDES_aarch64_linux_android="$INCLUDES_BASE:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/aarch64-linux-android"
    export AWS_LC_SYS_INCLUDES_armv7_linux_androideabi="$INCLUDES_BASE:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/arm-linux-androideabi"
    export AWS_LC_SYS_INCLUDES_i686_linux_android="$INCLUDES_BASE:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/i686-linux-android"
    export AWS_LC_SYS_INCLUDES_x86_64_linux_android="$INCLUDES_BASE:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot/usr/include/x86_64-linux-android"
fi

# Set sysroot for BINDGEN_EXTRA_CLANG_ARGS
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$NDK_PREBUILT_DIR/sysroot"

# Build the appropriate command based on build type
if [[ "$BUILD_TYPE" == "release" ]]; then
    echo "Building in RELEASE mode..."
    BUILD_COMMAND="yarn tauri android build"
else
    echo "Building in DEBUG mode..."
    BUILD_COMMAND="yarn tauri android build --debug"
fi

# Execute the build
echo "Executing: $BUILD_COMMAND"
if eval "$BUILD_COMMAND"; then
    echo ""
    echo "Build completed successfully!"
    
    if [[ "$BUILD_TYPE" == "release" ]]; then
        echo "Release APK: src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk"
        echo "Release AAB: src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab"
    else
        echo "Debug APK: src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk"
        echo "Debug AAB: src-tauri/gen/android/app/build/outputs/bundle/universalDebug/app-universal-debug.aab"
    fi
else
    echo ""
    echo "Build failed with exit code $?"
    exit $?
fi