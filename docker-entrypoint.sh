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
    echo "Railway hostname after cleanup: $RAILWAY_HOSTNAME"
    
    # Add the Railway hostname to the config
    # Find the line with www.rustybeam.net and add a new table row after it
    sed -i "/<span itemprop=\"hostname\" contenteditable=\"true\">www.rustybeam.net<\/span><\/td>/a\\                </tr>\\n                <tr>\\n                    <td class=\"ui-only\">Hostname</td>\\n                    <td colspan=\"2\"><span itemprop=\"hostname\" contenteditable=\"true\">$RAILWAY_HOSTNAME</span></td>" "$CONFIG_FILE"
    
    # Verify the change was made
    if grep -q "$RAILWAY_HOSTNAME" "$CONFIG_FILE"; then
        echo "Successfully added Railway hostname to config: $RAILWAY_HOSTNAME"
    else
        echo "WARNING: Failed to add Railway hostname to config!"
        echo "Attempting alternate method..."
        # Try a simpler approach - find the closing </tr> after www.rustybeam.net
        sed -i "/www.rustybeam.net<\/span><\/td>/{n;s|</tr>|</tr>\\n                <tr>\\n                    <td class=\"ui-only\">Hostname</td>\\n                    <td colspan=\"2\"><span itemprop=\"hostname\" contenteditable=\"true\">$RAILWAY_HOSTNAME</span></td>\\n                </tr>|}" "$CONFIG_FILE"
    fi
else
    echo "No Railway hostname detected, using default hostnames"
fi

# Note: OAuth2 callback URL is now configured via environment variable in the plugin config
# If GOOGLE_OAUTH2_CALLBACK is not set but we have a Railway hostname, suggest setting it
if [ -z "$GOOGLE_OAUTH2_CALLBACK" ] && [ ! -z "$RAILWAY_HOSTNAME" ]; then
    echo "INFO: Consider setting GOOGLE_OAUTH2_CALLBACK to: https://$RAILWAY_HOSTNAME/auth/google/callback"
fi

echo "Starting rusty-beam server..."
echo "Config file: $CONFIG_FILE"

# Debug: Show the hostnames that were configured
echo "Configured hostnames:"
grep 'itemprop="hostname"' "$CONFIG_FILE" | sed 's/.*contenteditable="true">\(.*\)<\/span>.*/  - \1/'

# Start the server with the config file in verbose mode (required for Docker)
# Docker needs a foreground process, and rusty-beam daemonizes unless -v is used
exec /app/rusty-beam -v "$CONFIG_FILE"