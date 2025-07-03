#!/bin/bash
set -e

echo "Building rusty-beam distribution packages..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${GREEN}[BUILD]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Clean previous builds
print_step "Cleaning previous builds..."
cargo clean
rm -rf target/packages
mkdir -p target/packages

# Build the main binary first
print_step "Building release binary..."
cargo build --release

# Build plugins
print_step "Building plugins..."
if [ -f "./build/scripts/build-plugins.sh" ]; then
    ./build/scripts/build-plugins.sh
else
    print_warning "build-plugins.sh not found, building plugins manually..."
    cd plugins/basic-auth && cargo build --release && cd ../..
    cd plugins/google-oauth2 && cargo build --release && cd ../..
    mkdir -p plugins/lib
    cp plugins/*/target/release/*.{so,dylib,dll} plugins/lib/ 2>/dev/null || true
fi

# Check if cargo-deb is installed
print_step "Building Debian package..."
if command -v cargo-deb &> /dev/null; then
    cargo deb --output target/packages/
    print_step "✓ Debian package created: $(ls target/packages/*.deb 2>/dev/null || echo 'FAILED')"
else
    print_warning "cargo-deb not installed. Install with: cargo install cargo-deb"
    print_warning "Skipping Debian package build."
fi

# Check if cargo-generate-rpm is installed
print_step "Building RPM package..."
if command -v cargo-generate-rpm &> /dev/null; then
    cargo generate-rpm --output target/packages/
    print_step "✓ RPM package created: $(ls target/packages/*.rpm 2>/dev/null || echo 'FAILED')"
else
    print_warning "cargo-generate-rpm not installed. Install with: cargo install cargo-generate-rpm"
    print_warning "Skipping RPM package build."
fi

# Create tarball for manual installation
print_step "Creating tarball..."
TARBALL_NAME="rusty-beam-$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "rusty-beam") | .version')-$(uname -s)-$(uname -m).tar.gz"
mkdir -p target/dist/rusty-beam
cp target/release/rusty-beam target/dist/rusty-beam/
cp config/config.html target/dist/rusty-beam/
cp -r plugins/lib target/dist/rusty-beam/plugins
cp -r examples/localhost target/dist/rusty-beam/examples-localhost
cp -r examples/files target/dist/rusty-beam/examples-files
cp README.md LICENSE target/dist/rusty-beam/
cd target/dist && tar -czf "../packages/$TARBALL_NAME" rusty-beam/ && cd ../..
print_step "✓ Tarball created: target/packages/$TARBALL_NAME"

# Create Windows ZIP (if building on Windows or cross-compiling)
if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    print_step "Creating Windows ZIP..."
    ZIP_NAME="rusty-beam-$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "rusty-beam") | .version')-windows.zip"
    cd target/dist && zip -r "../packages/$ZIP_NAME" rusty-beam/ && cd ../..
    print_step "✓ Windows ZIP created: target/packages/$ZIP_NAME"
fi

# Create AppImage (if on Linux and appimage-builder is available)
if command -v appimagetool &> /dev/null && [[ "$OSTYPE" == "linux-gnu"* ]]; then
    print_step "Building AppImage..."
    # This would require an AppImageBuilder.yml file and more setup
    print_warning "AppImage support requires additional setup - skipping for now"
fi

# List all created packages
print_step "Distribution packages created:"
ls -la target/packages/

print_step "Build complete! Packages are in target/packages/"

# Instructions for publishing
echo ""
echo "=== Publishing Instructions ==="
echo ""
echo "Debian/Ubuntu:"
echo "  Upload the .deb file to your repository or distribute directly"
echo ""
echo "RPM (RHEL/CentOS/Fedora):"
echo "  Upload the .rpm file to your repository or distribute directly"
echo ""
echo "Homebrew (macOS):"
echo "  1. Create a GitHub release with the tarball"
echo "  2. Update the SHA256 in homebrew/rusty-beam.rb"
echo "  3. Submit a PR to homebrew-core or create your own tap"
echo ""
echo "Manual installation:"
echo "  Extract the tarball and run the binary directly"