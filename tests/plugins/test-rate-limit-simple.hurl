# Rate Limit Plugin Tests - Simplified Version
# Tests basic rate limiting functionality

###############################################################################
# Basic Rate Limiting Tests
###############################################################################

# First few requests should succeed
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

# Note: The plugin is configured with 10 requests per minute
# We're not hitting the limit in this simplified test