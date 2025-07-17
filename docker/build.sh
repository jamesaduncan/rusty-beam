#!/bin/bash
# Build script for rusty-beam Docker images
# Supports building for multiple architectures

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
ARCH=""
IMAGE_NAME="rusty-beam"
IMAGE_TAG="latest"
PUSH=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --arch)
            ARCH="$2"
            shift 2
            ;;
        --name)
            IMAGE_NAME="$2"
            shift 2
            ;;
        --tag)
            IMAGE_TAG="$2"
            shift 2
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --arch ARCH    Build for specific architecture (amd64, arm64, or all)"
            echo "  --name NAME    Docker image name (default: rusty-beam)"
            echo "  --tag TAG      Docker image tag (default: latest)"
            echo "  --push         Push images to registry after building"
            echo "  --help         Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Determine architecture(s) to build
if [ -z "$ARCH" ]; then
    ARCH=$(dpkg --print-architecture)
    echo -e "${YELLOW}No architecture specified, building for native: $ARCH${NC}"
elif [ "$ARCH" = "all" ]; then
    ARCHITECTURES=("amd64" "arm64")
    echo -e "${GREEN}Building for all architectures: ${ARCHITECTURES[*]}${NC}"
else
    ARCHITECTURES=("$ARCH")
    echo -e "${GREEN}Building for architecture: $ARCH${NC}"
fi

# Function to build Debian package
build_debian_package() {
    local arch=$1
    echo -e "${YELLOW}Building Debian package for $arch...${NC}"
    
    # Go to parent directory to run the build script
    cd ..
    ./build-debian-package.sh --arch "$arch"
    cd docker
    
    # Find the built package
    local deb_file="../rusty-beam_*_${arch}.deb"
    if ! ls $deb_file 1> /dev/null 2>&1; then
        echo -e "${RED}Error: Debian package not found for $arch${NC}"
        return 1
    fi
    
    # Copy the package to docker directory
    cp $deb_file .
    echo -e "${GREEN}Debian package copied to docker directory${NC}"
}

# Function to build Docker image
build_docker_image() {
    local arch=$1
    local docker_arch=""
    
    # Map Debian architecture to Docker platform
    case "$arch" in
        "amd64")
            docker_arch="linux/amd64"
            ;;
        "arm64")
            docker_arch="linux/arm64"
            ;;
        *)
            echo -e "${RED}Unsupported architecture: $arch${NC}"
            return 1
            ;;
    esac
    
    # Find the Debian package
    local deb_file=$(ls rusty-beam_*_${arch}.deb 2>/dev/null | head -n1)
    if [ -z "$deb_file" ]; then
        echo -e "${RED}Error: Debian package not found in docker directory for $arch${NC}"
        echo -e "${YELLOW}Please run: build_debian_package $arch${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}Building Docker image for $arch...${NC}"
    echo -e "${YELLOW}Using Debian package: $deb_file${NC}"
    
    # Build the Docker image
    docker build \
        --platform "$docker_arch" \
        --build-arg DEB_FILE="$deb_file" \
        -t "${IMAGE_NAME}:${IMAGE_TAG}-${arch}" \
        -t "${IMAGE_NAME}:${arch}" \
        .
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Successfully built ${IMAGE_NAME}:${IMAGE_TAG}-${arch}${NC}"
        
        # Push if requested
        if [ "$PUSH" = true ]; then
            echo -e "${YELLOW}Pushing ${IMAGE_NAME}:${IMAGE_TAG}-${arch}...${NC}"
            docker push "${IMAGE_NAME}:${IMAGE_TAG}-${arch}"
            docker push "${IMAGE_NAME}:${arch}"
        fi
    else
        echo -e "${RED}Failed to build Docker image for $arch${NC}"
        return 1
    fi
}

# Main build process
main() {
    # Ensure we're in the docker directory
    if [ ! -f "Dockerfile" ]; then
        echo -e "${RED}Error: Must run from the docker/ directory${NC}"
        exit 1
    fi
    
    # Make entrypoint executable
    chmod +x docker-entrypoint.sh
    
    # Handle single architecture or "all"
    if [ "$ARCH" = "all" ]; then
        # Build for all architectures
        for arch in "${ARCHITECTURES[@]}"; do
            echo -e "\n${GREEN}=== Building for $arch ===${NC}"
            
            # Check if Debian package exists
            if ! ls rusty-beam_*_${arch}.deb 1> /dev/null 2>&1; then
                build_debian_package "$arch"
            else
                echo -e "${YELLOW}Debian package already exists for $arch${NC}"
            fi
            
            # Build Docker image
            build_docker_image "$arch"
        done
        
        # Create and push manifest if pushing
        if [ "$PUSH" = true ]; then
            echo -e "\n${YELLOW}Creating multi-arch manifest...${NC}"
            
            # Remove existing manifest if it exists
            docker manifest rm "${IMAGE_NAME}:${IMAGE_TAG}" 2>/dev/null || true
            
            # Create new manifest
            docker manifest create "${IMAGE_NAME}:${IMAGE_TAG}" \
                "${IMAGE_NAME}:${IMAGE_TAG}-amd64" \
                "${IMAGE_NAME}:${IMAGE_TAG}-arm64"
            
            docker manifest annotate "${IMAGE_NAME}:${IMAGE_TAG}" \
                "${IMAGE_NAME}:${IMAGE_TAG}-amd64" --arch amd64
            
            docker manifest annotate "${IMAGE_NAME}:${IMAGE_TAG}" \
                "${IMAGE_NAME}:${IMAGE_TAG}-arm64" --arch arm64
            
            docker manifest push "${IMAGE_NAME}:${IMAGE_TAG}"
            echo -e "${GREEN}Multi-arch manifest pushed${NC}"
        fi
    else
        # Build for single architecture
        for arch in "${ARCHITECTURES[@]}"; do
            # Check if Debian package exists
            if ! ls rusty-beam_*_${arch}.deb 1> /dev/null 2>&1; then
                build_debian_package "$arch"
            else
                echo -e "${YELLOW}Debian package already exists for $arch${NC}"
            fi
            
            # Build Docker image
            build_docker_image "$arch"
            
            # Tag as latest for single arch builds
            docker tag "${IMAGE_NAME}:${IMAGE_TAG}-${arch}" "${IMAGE_NAME}:${IMAGE_TAG}"
            
            if [ "$PUSH" = true ]; then
                docker push "${IMAGE_NAME}:${IMAGE_TAG}"
            fi
        done
    fi
    
    echo -e "\n${GREEN}Build complete!${NC}"
    echo -e "Images built:"
    docker images | grep "$IMAGE_NAME" | grep -E "(${IMAGE_TAG}|amd64|arm64)"
}

# Run main function
main