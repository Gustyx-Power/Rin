@echo off
setlocal enabledelayedexpansion

echo Building Rin for Android...

REM Check if cargo is installed
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/
    exit /b 1
)

REM Check if Android NDK is configured
if "%ANDROID_NDK_HOME%"=="" (
    if "%NDK_HOME%"=="" (
        echo Error: ANDROID_NDK_HOME or NDK_HOME environment variable not set
        echo Please set it to your NDK path, e.g.:
        echo set ANDROID_NDK_HOME=C:\Users\YourName\AppData\Local\Android\Sdk\ndk\26.1.10909125
        exit /b 1
    )
    set NDK_PATH=%NDK_HOME%
) else (
    set NDK_PATH=%ANDROID_NDK_HOME%
)

echo Using NDK: %NDK_PATH%

REM Set up NDK toolchain paths and environment variables
set PATH=%NDK_PATH%\toolchains\llvm\prebuilt\windows-x86_64\bin;%PATH%
set CC=%NDK_PATH%\toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe --target=aarch64-linux-android24
set AR=%NDK_PATH%\toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-ar.exe
set CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=%NDK_PATH%\toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe

REM Build for ARM64 (primary target)
echo.
echo Building for aarch64-linux-android...
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release --features android --lib
if %ERRORLEVEL% NEQ 0 (
    echo Build failed for aarch64-linux-android
    exit /b 1
)

REM Build rpkg CLI for ARM64
echo.
echo Building rpkg CLI for aarch64-linux-android...
cargo ndk -t arm64-v8a build --release --package rpkg --bin rpkg
if %ERRORLEVEL% NEQ 0 (
    echo Build failed for rpkg CLI
    exit /b 1
)

REM Copy and rename rpkg binary to librpkg_cli.so for Android
if not exist "android\app\src\main\jniLibs\arm64-v8a" mkdir "android\app\src\main\jniLibs\arm64-v8a"
copy /Y "target\aarch64-linux-android\release\rpkg" "android\app\src\main\jniLibs\arm64-v8a\librpkg_cli.so"

REM Build for ARMv7
echo.
echo Building for armv7-linux-androideabi...
cargo ndk -t armeabi-v7a -o android/app/src/main/jniLibs build --release --features android --lib
if %ERRORLEVEL% NEQ 0 (
    echo Build failed for armv7-linux-androideabi
    exit /b 1
)

REM Build rpkg CLI for ARMv7
echo.
echo Building rpkg CLI for armv7-linux-androideabi...
cargo ndk -t armeabi-v7a build --release --package rpkg --bin rpkg
if %ERRORLEVEL% NEQ 0 (
    echo Build failed for rpkg CLI
    exit /b 1
)

if not exist "android\app\src\main\jniLibs\armeabi-v7a" mkdir "android\app\src\main\jniLibs\armeabi-v7a"
copy /Y "target\armv7-linux-androideabi\release\rpkg" "android\app\src\main\jniLibs\armeabi-v7a\librpkg_cli.so"

REM Build for x86_64 (emulator)
echo.
echo Building for x86_64-linux-android...
cargo ndk -t x86_64 -o android/app/src/main/jniLibs build --release --features android --lib
if %ERRORLEVEL% NEQ 0 (
    echo Build failed for x86_64-linux-android
    exit /b 1
)

REM Build rpkg CLI for x86_64
echo.
echo Building rpkg CLI for x86_64-linux-android...
cargo ndk -t x86_64 build --release --package rpkg --bin rpkg
if %ERRORLEVEL% NEQ 0 (
    echo Build failed for rpkg CLI
    exit /b 1
)

if not exist "android\app\src\main\jniLibs\x86_64" mkdir "android\app\src\main\jniLibs\x86_64"
copy /Y "target\x86_64-linux-android\release\rpkg" "android\app\src\main\jniLibs\x86_64\librpkg_cli.so"

echo.
echo ========================================
echo Build complete! Libraries copied to jniLibs/
echo Now run: cd android ^&^& gradlew.bat assembleDebug
echo ========================================
