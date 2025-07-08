#!/bin/bash
# Build all plugins for rusty-beam

set -e

echo "Building rusty-beam plugins..."

# Create plugins directory if it doesn't exist
mkdir -p plugins

# Build each plugin
PLUGINS=(
    "selector-handler"
    "file-handler"
    "basic-auth"
    "authorization"
    "access-log"
    "compression"
    "cors"
    "error-handler"
    "health-check"
    "rate-limit"
    "redirect"
    "security-headers"
)

for plugin in "${PLUGINS[@]}"; do
    echo "Building $plugin..."
    cd "plugins/$plugin"
    cargo build --release
    cd ../..
    
    # Copy the built library to the plugins directory
    if [ -f "plugins/$plugin/target/release/librusty_beam_${plugin//-/_}.so" ]; then
        cp "plugins/$plugin/target/release/librusty_beam_${plugin//-/_}.so" "plugins/${plugin}.so"
        echo "✓ Built plugins/${plugin}.so"
    else
        echo "✗ Failed to find built library for $plugin"
        exit 1
    fi
done

# Special case for file-handler-v2
if [ -f "plugins/file-handler.so" ]; then
    cp "plugins/file-handler.so" "plugins/file-handler-v2.so"
    echo "✓ Created plugins/file-handler-v2.so"
fi

echo "All plugins built successfully!"