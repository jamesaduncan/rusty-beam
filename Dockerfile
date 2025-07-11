# Build stage
FROM rustlang/rust:nightly AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the entire project
COPY . .

# Build plugins first
RUN chmod +x build-plugins.sh && ./build-plugins.sh

# Build the main application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 rustybeam

# Create necessary directories
RUN mkdir -p /app/plugins /app/docs

# Copy the binary from builder
COPY --from=builder /app/target/release/rusty-beam /app/rusty-beam

# Copy plugins
COPY --from=builder /app/plugins/*.so /app/plugins/

# Copy the docs directory (which serves as the web root)
COPY --from=builder /app/docs /app/docs

# Copy the entrypoint script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# No need for config template since we mount it

# Set ownership
RUN chown -R rustybeam:rustybeam /app

# Switch to non-root user
USER rustybeam

# Set working directory
WORKDIR /app

# Expose the default port
EXPOSE 3000

# Use the entrypoint script
ENTRYPOINT ["/app/docker-entrypoint.sh"]