#!/usr/bin/env bash
# BlitzHash - MAXIMUM SPEED BUILD
# This builds with every optimization turned up to 11

set -e

echo "BLITZHASH - MAXIMUM SPEED BUILD"
echo

# Detect CPU and optimize for it
echo "🎯 Detecting CPU features..."
if command -v lscpu &> /dev/null; then
    echo "CPU: $(lscpu | grep 'Model name' | cut -d: -f2 | xargs)"
fi

# NUCLEAR RUST FLAGS
export RUSTFLAGS="\
-C target-cpu=native \
-C opt-level=3 \
-C lto=fat \
-C codegen-units=1 \
-C embed-bitcode=yes \
-C panic=abort \
-C prefer-dynamic=no \
-C link-arg=-fuse-ld=lld"

echo
echo "🚀 Build flags:"
echo "   ✅ target-cpu=native (use ALL CPU features)"
echo "   ✅ opt-level=3 (maximum optimization)"
echo "   ✅ lto=fat (link-time optimization)"
echo "   ✅ codegen-units=1 (maximum inlining)"
echo "   ✅ panic=abort (no unwinding overhead)"
echo

echo "🔨 Building BlitzHash..."
cargo build --release

echo
echo "✅ Build complete!"
echo
echo "🏃 Quick benchmark:"
cargo run --release --bin bench -- --size 100000000 --threads 8

echo
echo "FOR ABSOLUTE MAXIMUM SPEED, also try:"
echo "   export RUSTFLAGS=\"-C target-cpu=native -C opt-level=3 -C lto=fat\""
echo "   cargo build --release"
echo "   cargo run --release --bin bench -- --size 1000000000 --threads 16"