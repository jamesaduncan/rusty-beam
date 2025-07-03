#!/bin/bash
set -e

echo "Building RPM package for rusty-beam..."

# Install cargo-generate-rpm if not present
if ! command -v cargo-generate-rpm &> /dev/null; then
    echo "Installing cargo-generate-rpm..."
    cargo install cargo-generate-rpm
fi

# Build release binary first
cargo build --release

# Build plugins
./build/scripts/build-plugins.sh

# Create the RPM package
cargo generate-rpm --output target/packages/

echo "RPM package created successfully!"
echo "Install with: sudo rpm -i target/packages/rusty-beam-*.rpm"