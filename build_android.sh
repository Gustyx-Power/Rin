#!/bin/bash
set -e

echo "Building Rin for Android..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check if cargo-ndk is installed
if ! command -v cargo-ndk &> /dev/null; then
    echo "Error: cargo-ndk not found. Installing..."
    cargo install cargo-ndk
fi

# Check if Android NDK is configured
if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$NDK_HOME" ]; then
    echo "Error: ANDROID_NDK_HOME or NDK_HOME environment variable not set"
    echo "Please set it to your NDK path, e.g.:"
    echo "export ANDROID_NDK_HOME=/path/to/ndk/28.2.13676358"
    exit 1
fi

NDK_PATH="${ANDROID_NDK_HOME:-$NDK_HOME}"
echo "Using NDK: $NDK_PATH"

# Build for ARM64 (primary target)
echo ""
echo "Building for aarch64-linux-android..."
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release --features android --lib
if [ $? -ne 0 ]; then
    echo "Build failed for aarch64-linux-android"
    exit 1
fi

# Build rpkg CLI for ARM64
echo ""
echo "Building rpkg CLI for aarch64-linux-android..."
cargo ndk -t arm64-v8a build --release --package rpkg --bin rpkg
if [ $? -ne 0 ]; then
    echo "Build failed for rpkg CLI"
    exit 1
fi

# Copy and rename rpkg binary to librpkg_cli.so for Android
mkdir -p android/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/rpkg android/app/src/main/jniLibs/arm64-v8a/librpkg_cli.so

# Build for ARMv7
echo ""
echo "Building for armv7-linux-androideabi..."
cargo ndk -t armeabi-v7a -o android/app/src/main/jniLibs build --release --features android --lib
if [ $? -ne 0 ]; then
    echo "Build failed for armv7-linux-androideabi"
    exit 1
fi

# Build rpkg CLI for ARMv7
echo ""
echo "Building rpkg CLI for armv7-linux-androideabi..."
cargo ndk -t armeabi-v7a build --release --package rpkg --bin rpkg
if [ $? -ne 0 ]; then
    echo "Build failed for rpkg CLI"
    exit 1
fi

mkdir -p android/app/src/main/jniLibs/armeabi-v7a
cp target/armv7-linux-androideabi/release/rpkg android/app/src/main/jniLibs/armeabi-v7a/librpkg_cli.so

# Build for x86_64 (emulator)
echo ""
echo "Building for x86_64-linux-android..."
cargo ndk -t x86_64 -o android/app/src/main/jniLibs build --release --features android --lib
if [ $? -ne 0 ]; then
    echo "Build failed for x86_64-linux-android"
    exit 1
fi

# Build rpkg CLI for x86_64
echo ""
echo "Building rpkg CLI for x86_64-linux-android..."
cargo ndk -t x86_64 build --release --package rpkg --bin rpkg
if [ $? -ne 0 ]; then
    echo "Build failed for rpkg CLI"
    exit 1
fi

mkdir -p android/app/src/main/jniLibs/x86_64
cp target/x86_64-linux-android/release/rpkg android/app/src/main/jniLibs/x86_64/librpkg_cli.so

echo ""
echo "========================================"
echo "Build complete! Libraries copied to jniLibs/"
echo "Now run: cd android && ./gradlew assembleDebug"
echo "========================================"
