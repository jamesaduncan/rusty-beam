#!/bin/bash
# Run the guestbook demo with Google OAuth2 authentication

# Set default config file
CONFIG_FILE="${1:-config/guestbook.html}"

echo "========================================="
echo "Rusty Beam Guestbook with Google OAuth2"
echo "========================================="
echo ""
echo "Using config file: $CONFIG_FILE"
echo ""
echo "Before running, ensure you have:"
echo "1. Created a Google Cloud project"
echo "2. Enabled Google+ API"
echo "3. Created OAuth2 credentials"
echo "4. Updated your config file with your credentials"
echo ""
echo "Press Enter to continue or Ctrl+C to cancel..."
read

# Build plugins if needed
if [ ! -f "plugins/librusty_beam_google_oauth2.so" ]; then
    echo "Building OAuth2 plugin..."
    cd plugins/google-oauth2
    cargo build --release
    cd ../..
    cp plugins/google-oauth2/target/release/librusty_beam_google_oauth2.so plugins/
fi

# Check if credentials are configured
if grep -q "YOUR_GOOGLE_CLIENT_ID" "$CONFIG_FILE"; then
    echo ""
    echo "⚠️  WARNING: Google OAuth2 credentials not configured!"
    echo "Edit $CONFIG_FILE and replace:"
    echo "  - YOUR_GOOGLE_CLIENT_ID"
    echo "  - YOUR_GOOGLE_CLIENT_SECRET"
    echo ""
    echo "For testing without OAuth2, you can comment out the Google OAuth2 plugin section."
    echo ""
fi

echo "Starting guestbook server..."
echo "Access the guestbook at: http://localhost:3000"
echo ""

# Run the server
./target/release/rusty-beam "$CONFIG_FILE"