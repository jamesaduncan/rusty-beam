#!/bin/bash

# Build script for dynamic plugins

echo "Building dynamic authentication plugins..."

# Create plugins output directory
mkdir -p plugins/lib

# Build basic-auth plugin
echo "Building basic-auth plugin..."
cd plugins/basic-auth
cargo build --release
if [ $? -eq 0 ]; then
    cp target/release/libbasic_auth_plugin.so ../lib/libbasic_auth.so 2>/dev/null || \
    cp target/release/libbasic_auth_plugin.dylib ../lib/libbasic_auth.dylib 2>/dev/null || \
    cp target/release/basic_auth_plugin.dll ../lib/basic_auth.dll 2>/dev/null || \
    echo "Warning: Could not copy basic-auth plugin library"
else
    echo "Failed to build basic-auth plugin"
    exit 1
fi
cd ../..

# Build google-oauth2 plugin
echo "Building google-oauth2 plugin..."
cd plugins/google-oauth2
cargo build --release
if [ $? -eq 0 ]; then
    cp target/release/libgoogle_oauth2_plugin.so ../lib/libgoogle_oauth2.so 2>/dev/null || \
    cp target/release/libgoogle_oauth2_plugin.dylib ../lib/libgoogle_oauth2.dylib 2>/dev/null || \
    cp target/release/google_oauth2_plugin.dll ../lib/google_oauth2.dll 2>/dev/null || \
    echo "Warning: Could not copy google-oauth2 plugin library"
else
    echo "Failed to build google-oauth2 plugin"
    exit 1
fi
cd ../..

echo "Plugin build complete!"
echo "Built plugins are available in plugins/lib/"
ls -la plugins/lib/