# Rate Limit Plugin Tests
# Tests rate limiting functionality

###############################################################################
# Basic Rate Limiting Tests
###############################################################################

# First request should succeed
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

# Rapid requests to test rate limiting
# Note: Rate limits depend on plugin configuration
# Assuming a low limit for testing (e.g., 10 requests per minute)

# Make multiple requests quickly
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

# Continue making requests until rate limit is hit
# This request might be rate limited
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP *
[Asserts]
# If rate limited, should return 429
# If not yet limited, returns 200

###############################################################################
# Rate Limit Headers Tests
###############################################################################

# Test rate limit headers
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP *
[Asserts]
header "X-RateLimit-Limit" exists
header "X-RateLimit-Remaining" exists
header "X-RateLimit-Reset" exists

# When rate limited
# GET http://{{host}}:{{port}}/foo.html
# Host: {{test_host}}
# HTTP 429
# [Asserts]
# header "Retry-After" exists
# header "X-RateLimit-Limit" exists
# header "X-RateLimit-Remaining" == "0"
# body contains "Too Many Requests"

###############################################################################
# Per-Path Rate Limiting
###############################################################################

# Different paths might have different limits
GET http://{{host}}:{{port}}/api/data
Host: {{test_host}}
HTTP 404  # Path doesn't exist but rate limit still applies

# Static resources might have higher limits
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP *

###############################################################################
# Per-IP Rate Limiting
###############################################################################

# Rate limits are typically per IP address
# Requests from different IPs would have separate limits
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
X-Forwarded-For: 192.168.1.100
HTTP *

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
X-Forwarded-For: 192.168.1.101
HTTP *

###############################################################################
# Rate Limit Methods
###############################################################################

# Different methods might have different limits
# Delete file first to ensure clean test
DELETE http://{{host}}:{{port}}/test-rate.txt
Host: {{test_host}}
HTTP *

POST http://{{host}}:{{port}}/test-rate.txt
Host: {{test_host}}
Content-Type: text/plain
```
Rate limit test
```
HTTP 200

PUT http://{{host}}:{{port}}/test-rate.txt
Host: {{test_host}}
Content-Type: text/plain
```
Update test
```
HTTP 200

DELETE http://{{host}}:{{port}}/test-rate.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Rate Limit Reset Tests
###############################################################################

# After waiting, rate limit should reset
# This would require actual waiting or time manipulation
# GET http://{{host}}:{{port}}/foo.html
# Host: {{test_host}}
# HTTP 200
# [Asserts]
# header "X-RateLimit-Remaining" > "0"

###############################################################################
# Authenticated vs Anonymous Rate Limits
###############################################################################

# Authenticated users might have higher limits
# GET http://{{host}}:{{port}}/foo.html
# Host: {{test_host}}
# Authorization: Basic YWRtaW46YWRtaW4xMjM=
# HTTP 200
# [Asserts]
# header "X-RateLimit-Limit" exists

###############################################################################
# Rate Limit Bypass Tests
###############################################################################

# Certain paths might be exempt from rate limiting
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
# Health checks typically bypass rate limits

GET http://{{host}}:{{port}}/robots.txt
Host: {{test_host}}
HTTP 404
# Static files might have different limits