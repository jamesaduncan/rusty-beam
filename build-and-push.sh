#!/bin/bash
set -e

# Build the image locally
echo "Building Docker image..."
docker build -t rusty-beam:latest .

# Tag for your registry (replace with your username/org)
# For Docker Hub:
docker tag rusty-beam:latest yourusername/rusty-beam:latest

# For GitHub Container Registry:
# docker tag rusty-beam:latest ghcr.io/yourusername/rusty-beam:latest

# Push to registry
echo "Pushing to registry..."
# docker push yourusername/rusty-beam:latest

echo "Done! Update your Railway deployment to use the pre-built image."