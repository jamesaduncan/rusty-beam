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

# Build everything at once using workspace
RUN cargo build --release --workspace

# Copy plugin libraries to the expected location
RUN mkdir -p plugins && \
    cp target/release/librusty_beam_selector_handler.so plugins/ && \
    cp target/release/librusty_beam_file_handler.so plugins/ && \
    cp target/release/librusty_beam_basic_auth.so plugins/ && \
    cp target/release/librusty_beam_authorization.so plugins/ && \
    cp target/release/librusty_beam_access_log.so plugins/ && \
    cp target/release/librusty_beam_compression.so plugins/ && \
    cp target/release/librusty_beam_cors.so plugins/ && \
    cp target/release/librusty_beam_error_handler.so plugins/ && \
    cp target/release/librusty_beam_google_oauth2.so plugins/ && \
    cp target/release/librusty_beam_health_check.so plugins/ && \
    cp target/release/librusty_beam_rate_limit.so plugins/ && \
    cp target/release/librusty_beam_redirect.so plugins/ && \
    cp target/release/librusty_beam_security_headers.so plugins/ && \
    cp target/release/librusty_beam_websocket.so plugins/ && \
    cp target/release/libdirectory.so plugins/ && \
    cp target/release/librusty_beam_config_reload.so plugins/ && \
    cp plugins/librusty_beam_file_handler.so plugins/librusty_beam_file_handler_v2.so

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