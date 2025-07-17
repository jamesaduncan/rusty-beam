#!/bin/bash
# Script to build Debian package for rusty-beam

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command line arguments
ARCH=""
if [ "$1" = "--arch" ] && [ -n "$2" ]; then
    ARCH="$2"
    shift 2
fi

# Determine target architecture
if [ -z "$ARCH" ]; then
    ARCH=$(dpkg --print-architecture)
    echo -e "${GREEN}Building Debian package for rusty-beam (native: $ARCH)...${NC}"
else
    echo -e "${GREEN}Building Debian package for rusty-beam (cross-compiling for: $ARCH)...${NC}"
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "debian" ]; then
    echo -e "${RED}Error: Must run from rusty-beam root directory${NC}"
    exit 1
fi

# Map Debian architectures to Rust targets
case "$ARCH" in
    "amd64"|"x86_64")
        RUST_TARGET="x86_64-unknown-linux-gnu"
        DEB_ARCH="amd64"
        ;;
    "arm64"|"aarch64")
        RUST_TARGET="aarch64-unknown-linux-gnu"
        DEB_ARCH="arm64"
        ;;
    "armhf")
        RUST_TARGET="armv7-unknown-linux-gnueabihf"
        DEB_ARCH="armhf"
        ;;
    *)
        echo -e "${RED}Error: Unsupported architecture: $ARCH${NC}"
        echo -e "${YELLOW}Supported architectures: amd64, arm64, armhf${NC}"
        exit 1
        ;;
esac

# Check if cross-compiling
NATIVE_ARCH=$(dpkg --print-architecture)
if [ "$DEB_ARCH" != "$NATIVE_ARCH" ]; then
    echo -e "${YELLOW}Cross-compilation detected: $NATIVE_ARCH -> $DEB_ARCH${NC}"
    
    # Check if Rust target is installed
    if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
        echo -e "${YELLOW}Installing Rust target: $RUST_TARGET${NC}"
        rustup target add "$RUST_TARGET"
    fi
    
    # Set environment variables for cross-compilation
    export CARGO_TARGET_DIR="target-$DEB_ARCH"
    export CARGO_BUILD_TARGET="$RUST_TARGET"
    
    # Check for required cross-compilation tools
    case "$DEB_ARCH" in
        "amd64")
            if ! command -v x86_64-linux-gnu-gcc &> /dev/null; then
                echo -e "${YELLOW}Cross-compilation toolchain for x86_64 not found${NC}"
                echo -e "${YELLOW}Please install it with:${NC}"
                echo -e "${GREEN}sudo apt-get install gcc-x86-64-linux-gnu g++-x86-64-linux-gnu${NC}"
                exit 1
            fi
            export CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
            export CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++
            export AR_x86_64_unknown_linux_gnu=x86_64-linux-gnu-ar
            export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
            echo -e "${GREEN}Using x86_64 cross-compilation toolchain${NC}"
            ;;
        "armhf")
            if ! command -v arm-linux-gnueabihf-gcc &> /dev/null; then
                echo -e "${YELLOW}Cross-compilation toolchain for armhf not found${NC}"
                echo -e "${YELLOW}Please install it with:${NC}"
                echo -e "${GREEN}sudo apt-get install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf${NC}"
                exit 1
            fi
            export CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc
            export CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++
            export AR_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-ar
            export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
            echo -e "${GREEN}Using armhf cross-compilation toolchain${NC}"
            ;;
    esac
fi

# Clean any previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
rm -f *.deb *.dsc *.tar.* *.changes *.buildinfo *.ddeb
cargo clean || true

# Install build dependencies if needed
echo -e "${YELLOW}Checking build dependencies...${NC}"
if ! command -v dpkg-buildpackage &> /dev/null; then
    echo -e "${RED}Error: dpkg-dev is not installed. Install with: sudo apt-get install dpkg-dev${NC}"
    exit 1
fi

if ! command -v debuild &> /dev/null; then
    echo -e "${RED}Error: devscripts is not installed. Install with: sudo apt-get install devscripts${NC}"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed. Install Rust toolchain first.${NC}"
    exit 1
fi

# Build the package using fakeroot
echo -e "${YELLOW}Building package...${NC}"

# Export architecture for debian/rules
export DEB_HOST_ARCH="$DEB_ARCH"

# First build the binaries
echo -e "${YELLOW}Building binaries and plugins...${NC}"
debian/rules build

# Then create the package
echo -e "${YELLOW}Creating Debian package...${NC}"
fakeroot debian/rules binary

# Check if build was successful
if [ $? -eq 0 ]; then
    echo -e "${GREEN}Package built successfully!${NC}"
    echo -e "${GREEN}Package files created:${NC}"
    ls -la rusty-beam*${DEB_ARCH}.deb
    
    # Rename package if cross-compiled to make it clear
    if [ "$DEB_ARCH" != "$NATIVE_ARCH" ]; then
        # The package should already have the correct architecture in the name
        echo -e "${GREEN}Cross-compiled package for $DEB_ARCH architecture${NC}"
    fi
    
    echo -e "\n${YELLOW}To install the package:${NC}"
    echo "sudo dpkg -i rusty-beam_*_${DEB_ARCH}.deb"
    echo "sudo apt-get install -f  # To resolve any dependencies"
    
    echo -e "\n${YELLOW}To start the service:${NC}"
    echo "sudo systemctl start rusty-beam"
    echo "sudo systemctl enable rusty-beam  # To start on boot"
else
    echo -e "${RED}Package build failed!${NC}"
    exit 1
fi