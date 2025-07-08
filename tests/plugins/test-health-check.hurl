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
jsonpath "$.uptime" exists

# Test alternate health endpoint (if configured)
GET http://{{host}}:{{port}}/_health
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"

###############################################################################
# Health Check Methods
###############################################################################

# Test HEAD method on health endpoint
HEAD http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"

# Test other methods should fail
POST http://{{host}}:{{port}}/health
Host: {{test_host}}
Content-Type: text/plain
```
Should not modify health
```
HTTP 405

PUT http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 405

DELETE http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 405

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

# Test detailed health information
GET http://{{host}}:{{port}}/health?detailed=true
Host: {{test_host}}
HTTP 200
[Asserts]
jsonpath "$.status" exists
jsonpath "$.version" exists
jsonpath "$.uptime" exists
# Might include additional details like memory usage, connections, etc.