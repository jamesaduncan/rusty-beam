#!/bin/bash
# Test script to simulate Railway deployment

echo "Testing Railway deployment configuration..."

# Simulate Railway environment
export PORT=8080
export RAILWAY_PUBLIC_DOMAIN="https://myapp.railway.app"

# Build the image
echo "Building Docker image..."
docker build -t rusty-beam:railway-test .

# Run container with Railway env vars
echo "Running container with Railway environment..."
docker run -d --name railway-test \
  -e PORT=$PORT \
  -e RAILWAY_PUBLIC_DOMAIN=$RAILWAY_PUBLIC_DOMAIN \
  -p $PORT:$PORT \
  rusty-beam:railway-test

# Wait for startup
echo "Waiting for startup..."
sleep 5

# Check if server is running
echo "Checking server status..."
curl -s http://localhost:$PORT/ || echo "Server check failed"

# Check logs
echo "Container logs:"
docker logs railway-test | tail -20

# Cleanup
echo "Cleaning up..."
docker stop railway-test
docker rm railway-test

echo "Railway deployment test complete!"