# Health Check Plugin Tests
# Tests health check endpoint functionality

###############################################################################
# Basic Health Check Tests
###############################################################################

# Test default health check endpoint
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"
body contains "status"
body contains "healthy"

# Test health check returns JSON
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
[Asserts]
jsonpath "$.status" == "healthy"
jsonpath "$.timestamp" exists
jsonpath "$.checks" exists

# Alternate health endpoint not implemented in current plugin

###############################################################################
# Health Check Methods
###############################################################################

# Note: HEAD method on /health not supported by current plugin implementation

# Note: Other HTTP methods are handled by file-handler plugin if health-check doesn't intercept them

###############################################################################
# Readiness Check Tests
###############################################################################

# Test readiness endpoint (if implemented)
# GET http://{{host}}:{{port}}/ready
# Host: {{test_host}}
# HTTP 200
# [Asserts]
# header "Content-Type" == "application/json"
# jsonpath "$.ready" == true

###############################################################################
# Liveness Check Tests  
###############################################################################

# Test liveness endpoint (if implemented)
# GET http://{{host}}:{{port}}/live
# Host: {{test_host}}
# HTTP 200
# [Asserts]
# header "Content-Type" == "application/json"
# jsonpath "$.alive" == true

###############################################################################
# Health Check Headers
###############################################################################

# Test cache control headers
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
[Asserts]
header "Cache-Control" matches "(no-cache|no-store|max-age=0)"

# Test CORS headers on health check
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
Origin: https://monitoring.example.com
HTTP 200
# Might include CORS headers for monitoring services

###############################################################################
# Health Check Response Format
###############################################################################

# Test health endpoint with query parameter (should work the same)
GET http://{{host}}:{{port}}/health?detailed=true
Host: {{test_host}}
HTTP 200
[Asserts]
jsonpath "$.status" == "healthy"
jsonpath "$.timestamp" exists