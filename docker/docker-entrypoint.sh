#!/bin/bash
set -e

echo "Starting rusty-beam Docker container..."

# Create necessary directories
mkdir -p /app

# Clone repository if DOCS_GIT_REPO is set
if [ ! -z "$DOCS_GIT_REPO" ]; then
    echo "Cloning repository from: $DOCS_GIT_REPO"
    
    # Clone into a temporary directory first
    TEMP_DIR="/tmp/git-clone-$$"
    git clone "$DOCS_GIT_REPO" "$TEMP_DIR"
    
    if [ $? -eq 0 ]; then
        echo "Successfully cloned repository"
        
        # Copy contents to /app (excluding .git if needed)
        cp -r "$TEMP_DIR"/* /app/ 2>/dev/null || true
        cp -r "$TEMP_DIR"/.[^.]* /app/ 2>/dev/null || true
        
        # Clean up temp directory
        rm -rf "$TEMP_DIR"
    else
        echo "ERROR: Failed to clone repository"
        exit 1
    fi
else
    echo "No DOCS_GIT_REPO specified, using default /app structure"
fi

# Create hostname-based directory structure
# Parse HOSTNAME and ADDITIONAL_HOSTNAMES
ALL_HOSTNAMES="$HOSTNAME"
if [ ! -z "$ADDITIONAL_HOSTNAMES" ]; then
    ALL_HOSTNAMES="$ALL_HOSTNAMES,$ADDITIONAL_HOSTNAMES"
fi

# Create public directories for each hostname if they don't exist
IFS=',' read -ra HOSTS <<< "$ALL_HOSTNAMES"
for host in "${HOSTS[@]}"; do
    host=$(echo "$host" | tr -d ' ')  # Trim whitespace
    if [ ! -z "$host" ]; then
        # Only create directory if it doesn't already exist from git clone
        if [ ! -d "/app/$host/public" ]; then
            echo "Creating public directory for hostname: $host"
            mkdir -p "/app/$host/public"
        else
            echo "Directory already exists for hostname: $host"
        fi
    fi
done

# Determine configuration file location
# Use CONFIG_FILE environment variable if set, otherwise use default
if [ ! -z "$CONFIG_FILE" ]; then
    echo "Using custom config file: $CONFIG_FILE"
    # If it's a relative path, prepend /app/
    if [[ "$CONFIG_FILE" != /* ]]; then
        CONFIG_FILE="/app/$CONFIG_FILE"
    fi
else
    CONFIG_FILE="/app/config.html"
    echo "Using default config file: $CONFIG_FILE"
fi

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "ERROR: Configuration file not found: $CONFIG_FILE"
    exit 1
fi

echo "Configuring rusty-beam..."

# Update port
if [ ! -z "$PORT" ]; then
    echo "Setting port to: $PORT"
    sed -i "s|<td itemprop=\"port\">8080</td>|<td itemprop=\"port\">$PORT</td>|g" "$CONFIG_FILE"
fi

# Update bind address
if [ ! -z "$BIND_ADDRESS" ]; then
    echo "Setting bind address to: $BIND_ADDRESS"
    # Add bind address to config if not present
    if ! grep -q 'itemprop="bindAddress"' "$CONFIG_FILE"; then
        sed -i '/<td itemprop="port">/a\            <td itemprop="bindAddress">'"$BIND_ADDRESS"'</td>' "$CONFIG_FILE"
    else
        sed -i "s|<td itemprop=\"bindAddress\">.*</td>|<td itemprop=\"bindAddress\">$BIND_ADDRESS</td>|g" "$CONFIG_FILE"
    fi
fi

# Add host configurations
if [ ! -z "$ALL_HOSTNAMES" ]; then
    echo "Configuring hostnames..."
    
    # Create a temporary file for the host configurations
    HOST_CONFIG_FILE="/tmp/host_configs.html"
    echo "" > "$HOST_CONFIG_FILE"
    
    for host in "${HOSTS[@]}"; do
        host=$(echo "$host" | tr -d ' ')  # Trim whitespace
        if [ ! -z "$host" ]; then
            cat >> "$HOST_CONFIG_FILE" << EOF

    <table itemscope itemtype="http://rustybeam.net/HostConfig">
        <tr>
            <td itemprop="hostname">$host</td>
            <td itemprop="server_root">/app/$host/public</td>
        </tr>
    </table>
EOF
        fi
    done
    
    # Insert host configurations after the main ServerConfig table
    # First, find the line number of the closing </table> tag
    LINE_NUM=$(grep -n '</table>' "$CONFIG_FILE" | head -1 | cut -d: -f1)
    
    if [ ! -z "$LINE_NUM" ]; then
        # Split the file and insert the host configs
        head -n "$LINE_NUM" "$CONFIG_FILE" > /tmp/config_top.html
        tail -n +$((LINE_NUM + 1)) "$CONFIG_FILE" > /tmp/config_bottom.html
        cat /tmp/config_top.html "$HOST_CONFIG_FILE" /tmp/config_bottom.html > "$CONFIG_FILE"
        
        # Clean up temp files
        rm -f /tmp/config_top.html /tmp/config_bottom.html "$HOST_CONFIG_FILE"
    fi
fi

# Show final configuration
echo "Configuration:"
echo "- Port: ${PORT:-8080}"
echo "- Bind Address: ${BIND_ADDRESS:-0.0.0.0}"
echo "- Hostnames: $ALL_HOSTNAMES"
echo "- Plugin Path: $RUSTY_BEAM_PLUGIN_PATH"

# Start rusty-beam
echo "Starting rusty-beam server..."
exec /usr/bin/rusty-beam -v "$CONFIG_FILE"