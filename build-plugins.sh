#!/bin/bash
# Build all plugins for rusty-beam using workspace

set -e

echo "Building rusty-beam and all plugins using workspace..."
echo "This now builds everything in a single pass, sharing compilation of common dependencies!"
echo

# Create plugins directory if it doesn't exist
mkdir -p plugins

# Build everything in the workspace at once
# This is MUCH faster than building each plugin separately
# Handle cross-compilation if CARGO_BUILD_TARGET is set
if [ -n "$CARGO_BUILD_TARGET" ]; then
    echo "Cross-compiling for target: $CARGO_BUILD_TARGET"
    cargo build --release --workspace --target "$CARGO_BUILD_TARGET"
else
    cargo build --release --workspace
fi

echo "Copying plugin libraries to plugins directory..."

# Determine the release directory based on cross-compilation
if [ -n "$CARGO_BUILD_TARGET" ]; then
    if [ -n "$CARGO_TARGET_DIR" ]; then
        RELEASE_DIR="$CARGO_TARGET_DIR/$CARGO_BUILD_TARGET/release"
    else
        RELEASE_DIR="target/$CARGO_BUILD_TARGET/release"
    fi
else
    RELEASE_DIR="target/release"
fi

# Copy the built libraries from the unified target directory
PLUGINS=(
    "selector-handler"
    "file-handler"
    "basic-auth"
    "authorization"
    "access-log"
    "compression"
    "cors"
    "error-handler"
    "oauth2"
    "health-check"
    "rate-limit"
    "redirect"
    "security-headers"
    "websocket"
    "directory"
    "config-reload"
    "html-prettifier"
)

for plugin in "${PLUGINS[@]}"; do
    # Handle special cases for directory which has different lib name
    if [ "$plugin" = "directory" ]; then
        if [ -f "$RELEASE_DIR/lib${plugin}.so" ]; then
            cp "$RELEASE_DIR/lib${plugin}.so" "plugins/lib${plugin}.so"
            echo "✓ Copied plugins/lib${plugin}.so"
        else
            echo "✗ Failed to find built library for $plugin"
            exit 1
        fi
    else
        plugin_name="librusty_beam_${plugin//-/_}.so"
        if [ -f "$RELEASE_DIR/${plugin_name}" ]; then
            cp "$RELEASE_DIR/${plugin_name}" "plugins/${plugin_name}"
            echo "✓ Copied plugins/${plugin_name}"
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

echo "All plugins built successfully using unified workspace!"