# Build a lightweight Docker image using the pre-built Debian package
FROM debian:bookworm-slim

# Install runtime dependencies and git for cloning docs repository
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the Debian package - this expects the .deb file to be in the build context
# The build script will handle copying the appropriate architecture
ARG DEB_FILE=rusty-beam_0.3.1-1_amd64.deb
COPY ${DEB_FILE} /tmp/rusty-beam.deb

# Install the package
RUN dpkg -i /tmp/rusty-beam.deb || apt-get install -f -y \
    && rm /tmp/rusty-beam.deb

# Create app directory structure
RUN mkdir -p /app \
    && chown -R rusty-beam:rusty-beam /app

# Copy configuration and entrypoint
COPY config.html /app/config.html
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Set working directory
WORKDIR /app

# Environment variables with defaults
ENV PORT=8080 \
    BIND_ADDRESS=0.0.0.0 \
    HOSTNAME=localhost \
    DOCS_GIT_REPO="" \
    CONFIG_FILE="" \
    RUSTY_BEAM_PLUGIN_PATH=/usr/lib/rusty-beam/plugins

# Switch to non-root user (created by the Debian package)
USER rusty-beam

# Expose default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT}/ || exit 1

# Use the entrypoint script
ENTRYPOINT ["/app/docker-entrypoint.sh"]