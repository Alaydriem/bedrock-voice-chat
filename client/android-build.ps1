param(
    [Parameter(Position=0)]
    [ValidateSet("debug", "release", "--debug", "--release")]
    [string]$BuildType = "debug"
)

# Normalize the build type parameter (remove -- prefix if present)
$BuildType = $BuildType -replace "^--", ""

Write-Host "Starting Android build in $BuildType mode..." -ForegroundColor Green

# Set environment variables for build tools
$env:CMAKE_GENERATOR = "Ninja"
$env:CMAKE = "cmake"
$env:AWS_LC_SYS_CMAKE_GENERATOR = "Ninja"
$env:AWS_LC_SYS_CMAKE_BUILDER = "1"
$env:AWS_LC_SYS_EXTERNAL_BINDGEN = "1"

# Add NDK tools to PATH so cargo can find the linkers
$env:PATH = "$env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\bin;$env:PATH"

# Force all architectures to use API 27 toolchain for AAudio support
$env:CC_aarch64_linux_android = "aarch64-linux-android27-clang.cmd"
$env:CXX_aarch64_linux_android = "aarch64-linux-android27-clang++.cmd"
$env:AR_aarch64_linux_android = "llvm-ar.exe"
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = "aarch64-linux-android27-clang.cmd"

$env:CC_armv7_linux_androideabi = "armv7a-linux-androideabi27-clang.cmd"
$env:CXX_armv7_linux_androideabi = "armv7a-linux-androideabi27-clang++.cmd"
$env:AR_armv7_linux_androideabi = "llvm-ar.exe"
$env:CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER = "armv7a-linux-androideabi27-clang.cmd"

$env:CC_i686_linux_android = "i686-linux-android27-clang.cmd"
$env:CXX_i686_linux_android = "i686-linux-android27-clang++.cmd"
$env:AR_i686_linux_android = "llvm-ar.exe"
$env:CARGO_TARGET_I686_LINUX_ANDROID_LINKER = "i686-linux-android27-clang.cmd"

$env:CC_x86_64_linux_android = "x86_64-linux-android27-clang.cmd"
$env:CXX_x86_64_linux_android = "x86_64-linux-android27-clang++.cmd"
$env:AR_x86_64_linux_android = "llvm-ar.exe"
$env:CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER = "x86_64-linux-android27-clang.cmd"

# Set RUSTFLAGS for each target with library search paths
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS = "--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\aarch64-linux-android\27"
$env:CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RUSTFLAGS = "--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\arm-linux-androideabi\27"
$env:CARGO_TARGET_I686_LINUX_ANDROID_RUSTFLAGS = "--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\i686-linux-android\27"
$env:CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS = "--cfg s2n_quic_unstable -C link-arg=-lc++_shared -L $env:ANDROID_NDK_HOME\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\x86_64-linux-android\27"

# Set base includes (clang builtin headers and general sysroot)
$includesBase = "$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/lib/clang/20/include;$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/include"

# Set target-specific includes for each architecture
$env:AWS_LC_SYS_INCLUDES_aarch64_linux_android = "$includesBase;$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/include/aarch64-linux-android"
$env:AWS_LC_SYS_INCLUDES_armv7_linux_androideabi = "$includesBase;$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/include/arm-linux-androideabi"
$env:AWS_LC_SYS_INCLUDES_i686_linux_android = "$includesBase;$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/include/i686-linux-android"
$env:AWS_LC_SYS_INCLUDES_x86_64_linux_android = "$includesBase;$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot/usr/include/x86_64-linux-android"

# Set sysroot for BINDGEN_EXTRA_CLANG_ARGS
$env:BINDGEN_EXTRA_CLANG_ARGS = "--sysroot=$env:ANDROID_NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/sysroot"

# Build the appropriate command based on build type
if ($BuildType -eq "release") {
    Write-Host "Building in RELEASE mode..." -ForegroundColor Yellow
    $command = "yarn tauri android build"
} else {
    Write-Host "Building in DEBUG mode..." -ForegroundColor Yellow
    $command = "yarn tauri android build --debug"
}

# Execute the build
try {
    Invoke-Expression $command
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`nBuild completed successfully!" -ForegroundColor Green
        
        if ($BuildType -eq "release") {
            Write-Host "Release APK: src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release.apk" -ForegroundColor Cyan
            Write-Host "Release AAB: src-tauri\gen\android\app\build\outputs\bundle\universalRelease\app-universal-release.aab" -ForegroundColor Cyan
        } else {
            Write-Host "Debug APK: src-tauri\gen\android\app\build\outputs\apk\universal\debug\app-universal-debug.apk" -ForegroundColor Cyan
            Write-Host "Debug AAB: src-tauri\gen\android\app\build\outputs\bundle\universalDebug\app-universal-debug.aab" -ForegroundColor Cyan
        }
    } else {
        Write-Host "`nBuild failed with exit code $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} catch {
    Write-Host "`nBuild failed with error: $_" -ForegroundColor Red
    exit 1
}
