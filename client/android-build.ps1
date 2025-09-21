param(
    [Parameter(Position=0)]
    [ValidateSet("debug", "release", "--debug", "--release")]
    [string]$BuildType = "debug"
)

# Normalize the build type parameter (remove -- prefix if present)
$BuildType = $BuildType -replace "^--", ""

Write-Host "Starting Android build in $BuildType mode..." -ForegroundColor Green

$env:LIBCLANG_PATH = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\Llvm\\x64\\lib"

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

# Function to copy Android icons
function Copy-AndroidIcons {
    Write-Host "Copying custom Android icons..." -ForegroundColor Yellow
    
    $sourceIconsPath = "src-tauri\icons\android"
    $targetResPath = "src-tauri\gen\android\app\src\main\res"
    
    if (-not (Test-Path $sourceIconsPath)) {
        Write-Host "Warning: Source icons path not found: $sourceIconsPath" -ForegroundColor Yellow
        return
    }
    
    if (-not (Test-Path $targetResPath)) {
        Write-Host "Warning: Target Android resources path not found: $targetResPath" -ForegroundColor Yellow
        Write-Host "This is normal if the Android project hasn't been initialized yet." -ForegroundColor Yellow
        return
    }
    
    # Copy each mipmap directory
    $mipmapDirs = @("mipmap-hdpi", "mipmap-mdpi", "mipmap-xhdpi", "mipmap-xxhdpi", "mipmap-xxxhdpi")
    
    foreach ($mipmapDir in $mipmapDirs) {
        $sourcePath = Join-Path $sourceIconsPath $mipmapDir
        $targetPath = Join-Path $targetResPath $mipmapDir
        
        if (Test-Path $sourcePath) {
            if (-not (Test-Path $targetPath)) {
                New-Item -ItemType Directory -Path $targetPath -Force | Out-Null
            }
            
            # Copy ic_launcher.png
            $sourceIcon = Join-Path $sourcePath "ic_launcher.png"
            $targetIcon = Join-Path $targetPath "ic_launcher.png"
            
            if (Test-Path $sourceIcon) {
                Copy-Item -Path $sourceIcon -Destination $targetIcon -Force
                Write-Host "Copied $mipmapDir/ic_launcher.png" -ForegroundColor Green
            } else {
                Write-Host "Missing source icon: $sourceIcon" -ForegroundColor Red
            }
        } else {
            Write-Host "Missing source directory: $sourcePath" -ForegroundColor Red
        }
    }
    
    Write-Host "Android icon copying completed." -ForegroundColor Green
}

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
    # First, ensure Android project is initialized
    Write-Host "Ensuring Android project is initialized..." -ForegroundColor Yellow
    yarn tauri android init
    
    # Copy custom Android icons after initialization
    Copy-AndroidIcons
    
    # Now execute the actual build
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
