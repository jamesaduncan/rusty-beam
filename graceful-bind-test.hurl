# Test for graceful failure when socket binding fails
# ✅ FIXED: Server now handles port conflicts gracefully

# Automated test: ./test-bind-failure.sh

# Expected behavior (now implemented):
# - First server starts successfully
# - Second server displays a clear error message about port being in use
# - Second server exits cleanly with exit code 1
# - No panic or stack trace is displayed
# - Helpful suggestions are provided to resolve the issue

# Example output:
# Failed to start server on 127.0.0.1:3000
# Error: error creating server listener: Address already in use (os error 98)
# It looks like another server is already running on this port.
# Please either:
#   - Stop the other server
#   - Change the port in config.html
#   - Use a different bind address