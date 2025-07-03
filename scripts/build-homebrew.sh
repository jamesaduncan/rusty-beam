#!/bin/bash
set -e

echo "Preparing Homebrew formula for rusty-beam..."

VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "rusty-beam") | .version')

echo "Building tarball for Homebrew..."

# Build release binary
cargo build --release

# Build plugins
./build-plugins.sh

# Create source tarball (Homebrew prefers to build from source)
TARBALL_NAME="rusty-beam-${VERSION}.tar.gz"
git archive --format=tar.gz --prefix="rusty-beam-${VERSION}/" HEAD > "target/packages/${TARBALL_NAME}"

# Calculate SHA256
SHA256=$(shasum -a 256 "target/packages/${TARBALL_NAME}" | cut -d' ' -f1)

echo "Created tarball: target/packages/${TARBALL_NAME}"
echo "SHA256: ${SHA256}"

# Update the Homebrew formula
sed -i.bak "s/REPLACE_WITH_ACTUAL_SHA256/${SHA256}/g" homebrew/rusty-beam.rb

echo ""
echo "Homebrew formula updated!"
echo "Next steps:"
echo "1. Create a GitHub release with the tarball"
echo "2. Update the URL in homebrew/rusty-beam.rb to point to the release"
echo "3. Test the formula with: brew install --build-from-source homebrew/rusty-beam.rb"
echo "4. Submit to homebrew-core or create your own tap"