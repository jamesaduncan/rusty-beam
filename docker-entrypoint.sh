#!/bin/bash
set -e

# Use the config from docs/config/index.html
CONFIG_FILE="/app/docs/config/index.html"

# For Docker, we need to bind to 0.0.0.0 instead of 127.0.0.1
echo "Updating bind address for Docker..."
sed -i 's|<span itemprop="bindAddress" contenteditable="true">127.0.0.1</span>|<span itemprop="bindAddress" contenteditable="true">0.0.0.0</span>|g' "$CONFIG_FILE"

echo "Starting rusty-beam server..."
echo "Config file: $CONFIG_FILE"

# Start the server with the config file in verbose mode (required for Docker)
# Docker needs a foreground process, and rusty-beam daemonizes unless -v is used
exec /app/rusty-beam -v "$CONFIG_FILE"