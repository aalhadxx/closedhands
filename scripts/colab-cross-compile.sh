#!/bin/bash
# Cross-compile ClosedHands (Grok Build fork) for Windows on Google Colab
# Paste each cell into Colab. Upload the repo as a zip first.

# === CELL 1: Install Rust & Windows target ===
!curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
import os
os.environ['PATH'] += ':/root/.cargo/bin'
!rustc --version
!rustup target add x86_64-pc-windows-gnu

# === CELL 2: Install MinGW cross-compiler ===
!apt-get update -qq && apt-get install -y -qq gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64

# === CELL 3: Set env vars for cross-compile ===
import os
os.environ['CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER'] = 'x86_64-w64-mingw32-gcc'
os.environ['CC_x86_64_pc_windows_gnu'] = 'x86_64-w64-mingw32-gcc'
os.environ['CXX_x86_64_pc_windows_gnu'] = 'x86_64-w64-mingw32-g++'

# === CELL 4: Upload & extract repo ===
# Use Colab file upload panel to upload closedhands.zip, then:
!unzip -q closedhands.zip -d /content/closedhands
%cd /content/closedhands

# === CELL 5: Replace /dev paths for Windows (patch xai-proto-build) ===
!sed -i 's|/dev/stdout|/tmp/protoc_out|g' crates/build/xai-proto-build/src/lib.rs 2>/dev/null || true
!sed -i 's|/dev/null|/tmp/protoc_null|g' crates/build/xai-proto-build/src/lib.rs 2>/dev/null || true

# === CELL 6: Build Windows binary ===
# Use --release for optimized build. Adjust -j if Colab RAM is tight (default -j 2).
!cargo build --target x86_64-pc-windows-gnu --release -j 2 2>&1 | tee build.log

# === CELL 7: Verify & prepare download ===
!ls -lh target/x86_64-pc-windows-gnu/release/*.exe 2>/dev/null || echo "No .exe found, check build.log"
!file target/x86_64-pc-windows-gnu/release/*.exe 2>/dev/null || true

# If the binary is named 'grok' or similar, rename it:
# !mv target/x86_64-pc-windows-gnu/release/grok target/x86_64-pc-windows-gnu/release/closedhands.exe

# === CELL 8: Download binary ===
from google.colab import files
files.download('/content/closedhands/target/x86_64-pc-windows-gnu/release/grok.exe')
