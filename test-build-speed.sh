#!/bin/bash

echo "=== Build Speed Comparison ==="
echo

# Clean everything first
cargo clean

echo "1. Testing OLD approach (separate builds)..."
echo "   Building just 3 plugins separately as an example..."
time (
    cd plugins/selector-handler && cargo build --release --quiet && cd ../..
    cd plugins/file-handler && cargo build --release --quiet && cd ../..
    cd plugins/basic-auth && cargo build --release --quiet && cd ../..
)

echo
echo "2. Cleaning and testing NEW approach (workspace)..."
cargo clean

echo "   Building ALL plugins and main app with workspace..."
time cargo build --release --workspace --quiet

echo
echo "=== Results ==="
echo "The workspace approach builds everything at once, sharing compilation"
echo "of common dependencies like tokio, hyper, serde, etc."