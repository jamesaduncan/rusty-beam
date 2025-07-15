#!/bin/bash
# Script to build Debian package for rusty-beam

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Debian package for rusty-beam...${NC}"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "debian" ]; then
    echo -e "${RED}Error: Must run from rusty-beam root directory${NC}"
    exit 1
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
    ls -la *.deb
    
    echo -e "\n${YELLOW}To install the package:${NC}"
    echo "sudo dpkg -i rusty-beam_*.deb"
    echo "sudo apt-get install -f  # To resolve any dependencies"
    
    echo -e "\n${YELLOW}To start the service:${NC}"
    echo "sudo systemctl start rusty-beam"
    echo "sudo systemctl enable rusty-beam  # To start on boot"
else
    echo -e "${RED}Package build failed!${NC}"
    exit 1
fi