# CORS Plugin Tests
# Tests Cross-Origin Resource Sharing functionality

###############################################################################
# Preflight Request Tests (OPTIONS)
###############################################################################

# Test basic preflight request
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
Access-Control-Request-Method: GET
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists
header "Access-Control-Allow-Methods" exists
header "Access-Control-Max-Age" exists

# Test preflight with specific method
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
Access-Control-Request-Method: PUT
Access-Control-Request-Headers: Content-Type, X-Custom-Header
HTTP 200
[Asserts]
header "Access-Control-Allow-Methods" contains "PUT"
header "Access-Control-Allow-Headers" exists

# Test preflight with unsupported method
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
Access-Control-Request-Method: TRACE
HTTP 200
# Should still return CORS headers even if method not supported

###############################################################################
# Simple CORS Request Tests
###############################################################################

# Test GET with Origin header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists
header "Vary" contains "Origin"

# Test POST with Origin
POST http://{{host}}:{{port}}/test-cors.txt
Host: {{test_host}}
Origin: https://example.com
Content-Type: text/plain
```
CORS test content
```
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test PUT with Origin
PUT http://{{host}}:{{port}}/test-cors.txt
Host: {{test_host}}
Origin: https://example.com
Content-Type: text/plain
```
Updated CORS content
```
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test DELETE with Origin
DELETE http://{{host}}:{{port}}/test-cors.txt
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

###############################################################################
# Origin Validation Tests
###############################################################################

# Test allowed origin
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://allowed-origin.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" matches "^(https://allowed-origin\\.com|\\*)$"

# Test wildcard origin (if configured)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://any-origin.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test no Origin header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Should work without CORS headers

###############################################################################
# Credentials Tests
###############################################################################

# Test with credentials
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
Cookie: session=abc123
HTTP 200
[Asserts]
header "Access-Control-Allow-Credentials" exists

# Test preflight with credentials
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
Access-Control-Request-Method: GET
Access-Control-Request-Headers: Cookie
HTTP 200
[Asserts]
header "Access-Control-Allow-Credentials" exists

###############################################################################
# Custom Headers Tests
###############################################################################

# Test with custom headers
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
X-Custom-Header: test-value
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test exposed headers
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Expose-Headers" exists

###############################################################################
# HTTP Methods Tests
###############################################################################

# Test HEAD with CORS
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

###############################################################################
# Multiple Origins Tests
###############################################################################

# Test with multiple origin values (invalid)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example1.com https://example2.com
HTTP 200
# Server should handle invalid multiple origins

###############################################################################
# Case Sensitivity Tests
###############################################################################

# Test case sensitivity of Origin
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: HTTPS://EXAMPLE.COM
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test case of CORS headers
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
origin: https://example.com
access-control-request-method: GET
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

###############################################################################
# Protocol Tests
###############################################################################

# Test HTTP origin
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: http://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test HTTPS origin
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test non-standard port
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com:8443
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

###############################################################################
# Error Response CORS
###############################################################################

# Test CORS on 404 response
GET http://{{host}}:{{port}}/non-existent.html
Host: {{test_host}}
Origin: https://example.com
HTTP 404
[Asserts]
header "Access-Control-Allow-Origin" exists

# Test CORS on 403 response
GET http://{{host}}:{{port}}/../etc/passwd
Host: {{test_host}}
Origin: https://example.com
HTTP 404
[Asserts]
header "Access-Control-Allow-Origin" exists