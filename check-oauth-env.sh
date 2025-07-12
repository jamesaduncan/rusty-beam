#!/bin/bash
# Script to check OAuth2 environment variables

echo "Checking OAuth2 Environment Variables..."
echo "========================================"
echo ""

# Google OAuth2
echo "Google OAuth2:"
if [ -z "$GOOGLE_CLIENT_ID" ]; then
    echo "  ❌ GOOGLE_CLIENT_ID is not set"
else
    echo "  ✓ GOOGLE_CLIENT_ID is set (length: ${#GOOGLE_CLIENT_ID})"
fi

if [ -z "$GOOGLE_CLIENT_SECRET" ]; then
    echo "  ❌ GOOGLE_CLIENT_SECRET is not set"
else
    echo "  ✓ GOOGLE_CLIENT_SECRET is set (length: ${#GOOGLE_CLIENT_SECRET})"
fi

if [ -z "$GOOGLE_OAUTH2_CALLBACK" ]; then
    echo "  ❌ GOOGLE_OAUTH2_CALLBACK is not set"
else
    echo "  ✓ GOOGLE_OAUTH2_CALLBACK = $GOOGLE_OAUTH2_CALLBACK"
fi

echo ""

# GitHub OAuth2
echo "GitHub OAuth2:"
if [ -z "$GITHUB_CLIENT_ID" ]; then
    echo "  ❌ GITHUB_CLIENT_ID is not set"
else
    echo "  ✓ GITHUB_CLIENT_ID is set (length: ${#GITHUB_CLIENT_ID})"
fi

if [ -z "$GITHUB_CLIENT_SECRET" ]; then
    echo "  ❌ GITHUB_CLIENT_SECRET is not set"
else
    echo "  ✓ GITHUB_CLIENT_SECRET is set (length: ${#GITHUB_CLIENT_SECRET})"
fi

if [ -z "$GITHUB_OAUTH2_CALLBACK" ]; then
    echo "  ❌ GITHUB_OAUTH2_CALLBACK is not set"
    # Check for common typo
    if [ ! -z "$GIT_OAUTH2_CALLBACK" ]; then
        echo "  ⚠️  Found GIT_OAUTH2_CALLBACK instead - this should be GITHUB_OAUTH2_CALLBACK"
        echo "     Value: $GIT_OAUTH2_CALLBACK"
    fi
else
    echo "  ✓ GITHUB_OAUTH2_CALLBACK = $GITHUB_OAUTH2_CALLBACK"
fi

echo ""
echo "Suggested fix for GitHub callback:"
echo "export GITHUB_OAUTH2_CALLBACK='$GIT_OAUTH2_CALLBACK'"