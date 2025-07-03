#!/bin/bash
set -e

echo "Building Debian package for rusty-beam..."

# Install cargo-deb if not present
if ! command -v cargo-deb &> /dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb
fi

# Build release binary first
cargo build --release

# Build plugins
./build/scripts/build-plugins.sh

# Create the Debian package
cargo deb --output target/packages/

echo "Debian package created successfully!"
echo "Install with: sudo dpkg -i target/packages/rusty-beam_*.deb"