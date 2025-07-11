#!/bin/bash
set -e

# Use the config from docs/config/index.html
CONFIG_FILE="/app/docs/config/index.html"

# For Docker, we need to bind to 0.0.0.0 instead of 127.0.0.1
echo "Updating bind address for Docker..."
sed -i 's|<span itemprop="bindAddress" contenteditable="true">127.0.0.1</span>|<span itemprop="bindAddress" contenteditable="true">0.0.0.0</span>|g' "$CONFIG_FILE"

# Update port if PORT environment variable is set (Railway sets this)
if [ ! -z "$PORT" ]; then
    echo "Updating port to $PORT..."
    sed -i "s|<span itemprop=\"bindPort\" contenteditable=\"true\">3000</span>|<span itemprop=\"bindPort\" contenteditable=\"true\">$PORT</span>|g" "$CONFIG_FILE"
    sed -i "s|<span itemprop=\"port\">3000</span>|<span itemprop=\"port\">$PORT</span>|g" "$CONFIG_FILE"
fi

# Railway provides the hostname in RAILWAY_PUBLIC_DOMAIN
# Also check for RAILWAY_STATIC_URL as a fallback
RAILWAY_HOSTNAME="${RAILWAY_PUBLIC_DOMAIN:-${RAILWAY_STATIC_URL:-}}"

if [ ! -z "$RAILWAY_HOSTNAME" ]; then
    echo "Railway hostname detected: $RAILWAY_HOSTNAME"
    
    # Remove https:// prefix if present
    RAILWAY_HOSTNAME=$(echo "$RAILWAY_HOSTNAME" | sed 's|^https://||')
    
    # Add the Railway hostname to the config
    # Find the line with rustybeam.net and add Railway hostname after it
    sed -i "/<span itemprop=\"hostname\">rustybeam.net<\/span>/a\\                <div>Hostname: <span itemprop=\"hostname\">$RAILWAY_HOSTNAME</span></div>" "$CONFIG_FILE"
    
    echo "Added Railway hostname to config: $RAILWAY_HOSTNAME"
else
    echo "No Railway hostname detected, using default hostnames"
fi

echo "Starting rusty-beam server..."
echo "Config file: $CONFIG_FILE"

# Start the server with the config file in verbose mode (required for Docker)
# Docker needs a foreground process, and rusty-beam daemonizes unless -v is used
exec /app/rusty-beam -v "$CONFIG_FILE"