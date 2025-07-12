#!/bin/bash
# Test OAuth2 startup with environment variables

echo "Testing OAuth2 Plugin Startup..."
echo "================================"
echo ""

# First, let's fix the typo
if [ ! -z "$GIT_OAUTH2_CALLBACK" ] && [ -z "$GITHUB_OAUTH2_CALLBACK" ]; then
    echo "Fixing environment variable name..."
    export GITHUB_OAUTH2_CALLBACK="$GIT_OAUTH2_CALLBACK"
    echo "✓ Set GITHUB_OAUTH2_CALLBACK from GIT_OAUTH2_CALLBACK"
    echo ""
fi

# Display current environment variables (masked for security)
echo "Current OAuth2 Environment Variables:"
echo "------------------------------------"
echo "GOOGLE_CLIENT_ID: ${GOOGLE_CLIENT_ID:+[SET - ${#GOOGLE_CLIENT_ID} chars]}"
echo "GOOGLE_CLIENT_SECRET: ${GOOGLE_CLIENT_SECRET:+[SET - ${#GOOGLE_CLIENT_SECRET} chars]}"
echo "GOOGLE_OAUTH2_CALLBACK: ${GOOGLE_OAUTH2_CALLBACK:-[NOT SET]}"
echo ""
echo "GITHUB_CLIENT_ID: ${GITHUB_CLIENT_ID:+[SET - ${#GITHUB_CLIENT_ID} chars]}"
echo "GITHUB_CLIENT_SECRET: ${GITHUB_CLIENT_SECRET:+[SET - ${#GITHUB_CLIENT_SECRET} chars]}"
echo "GITHUB_OAUTH2_CALLBACK: ${GITHUB_OAUTH2_CALLBACK:-[NOT SET]}"
echo ""

# Create a test configuration
cat > /tmp/oauth-test-config.html << 'EOF'
<!DOCTYPE html>
<html>
<body itemscope itemtype="http://rustybeam.net/ServerConfig">
    <table>
        <tr>
            <td>Bind Address</td>
            <td><span itemprop="bindAddress">127.0.0.1</span></td>
        </tr>
        <tr>
            <td>Port</td>
            <td><span itemprop="bindPort">3456</span></td>
        </tr>
    </table>
    
    <table itemscope itemtype="http://rustybeam.net/HostConfig">
        <tr>
            <td>Hostname</td>
            <td><span itemprop="hostname">localhost</span></td>
        </tr>
        <tr>
            <td>Host Root</td>
            <td><span itemprop="hostRoot">./docs</span></td>
        </tr>
        <tr>
            <td>Google OAuth2</td>
            <td itemprop="plugin" itemscope itemtype="http://rustybeam.net/OAuth2Plugin">
                <span itemprop="library">file://./plugins/librusty_beam_oauth2.so</span>
                <span itemprop="name">google-oauth2</span>
                <span itemprop="clientIdEnv">GOOGLE_CLIENT_ID</span>
                <span itemprop="clientSecretEnv">GOOGLE_CLIENT_SECRET</span>
                <span itemprop="redirectUriEnv">GOOGLE_OAUTH2_CALLBACK</span>
                <span itemprop="loginPath">/auth/google/login</span>
            </td>
        </tr>
        <tr>
            <td>File Handler</td>
            <td itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                <span itemprop="library">file://./plugins/librusty_beam_file_handler.so</span>
            </td>
        </tr>
    </table>
</body>
</html>
EOF

echo "Starting server with test config..."
echo "Watch for Warning messages about environment variables..."
echo ""
echo "========== SERVER OUTPUT =========="
timeout 5s cargo run -- /tmp/oauth-test-config.html 2>&1 | grep -E "Warning:|Error:|PID:|Successfully loaded plugin|OAuth2"
echo "========== END OUTPUT =========="
echo ""

# Clean up
rm -f /tmp/oauth-test-config.html

echo ""
echo "If you see warnings about environment variables not being set,"
echo "make sure to export them before running the server:"
echo ""
echo "export GOOGLE_CLIENT_ID='your-client-id'"
echo "export GOOGLE_CLIENT_SECRET='your-client-secret'"
echo "export GOOGLE_OAUTH2_CALLBACK='http://localhost:3000/auth/google/callback'"
echo "export GITHUB_CLIENT_ID='your-github-client-id'"
echo "export GITHUB_CLIENT_SECRET='your-github-client-secret'"
echo "export GITHUB_OAUTH2_CALLBACK='http://localhost:3000/auth/github/callback'"