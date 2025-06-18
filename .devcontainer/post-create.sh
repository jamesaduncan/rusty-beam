#!/bin/bash
# .devcontainer/post-create.sh
set -e

echo "🦀 Setting up Rust development environment..."

# Update Rust to latest stable
rustup update stable
rustup default stable

# Add common Rust components
rustup component add rustfmt clippy rust-src rust-analyzer

# Install common Rust tools
cargo install cargo-watch cargo-edit cargo-audit cargo-outdated cargo-tree

# Install additional useful tools
cargo install bat exa fd-find ripgrep tokei
cargo install cargo-generate cargo-expand cargo-udeps

# Setup git hooks (if .git exists)
if [ -d ".git" ]; then
    echo "Setting up git hooks..."
    # You can add pre-commit hooks here
fi

echo "✅ Rust development environment setup complete!"
