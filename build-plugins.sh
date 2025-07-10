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
    "google-oauth2"
    "health-check"
    "rate-limit"
    "redirect"
    "security-headers"
    "websocket"
    "directory"
)

for plugin in "${PLUGINS[@]}"; do
    echo "Building $plugin..."
    cd "plugins/$plugin"
    cargo build --release
    cd ../..
    
    # Copy the built library to the plugins directory
    # Handle special cases for directory which has different lib name
    if [ "$plugin" = "directory" ]; then
        if [ -f "plugins/$plugin/target/release/lib${plugin}.so" ]; then
            cp "plugins/$plugin/target/release/lib${plugin}.so" "plugins/lib${plugin}.so"
            echo "✓ Built plugins/lib${plugin}.so"
        else
            echo "✗ Failed to find built library for $plugin"
            exit 1
        fi
    else
        if [ -f "plugins/$plugin/target/release/librusty_beam_${plugin//-/_}.so" ]; then
            cp "plugins/$plugin/target/release/librusty_beam_${plugin//-/_}.so" "plugins/librusty_beam_${plugin//-/_}.so"
            echo "✓ Built plugins/librusty_beam_${plugin//-/_}.so"
        else
            echo "✗ Failed to find built library for $plugin"
            exit 1
        fi
    fi
done

# Special case for file-handler-v2
if [ -f "plugins/librusty_beam_file_handler.so" ]; then
    cp "plugins/librusty_beam_file_handler.so" "plugins/librusty_beam_file_handler_v2.so"
    echo "✓ Created plugins/librusty_beam_file_handler_v2.so"
fi

echo "All plugins built successfully!"