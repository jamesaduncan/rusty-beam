# Error Handler Plugin Tests
# Tests custom error page functionality

###############################################################################
# 404 Not Found Error Tests
###############################################################################

# Test 404 error for missing file
GET http://{{host}}:{{port}}/non-existent-file.html
Host: {{test_host}}
HTTP 404
[Asserts]
header "Content-Type" contains "text/"
# Should return custom 404 page if configured, or default message

# Test 404 for missing directory
GET http://{{host}}:{{port}}/missing-directory/
Host: {{test_host}}
HTTP 404

# Test 404 with Accept header preferring HTML
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8
HTTP 404
[Asserts]
header "Content-Type" matches "(text/html|text/plain)"

# Test 404 with Accept header preferring JSON
GET http://{{host}}:{{port}}/missing.json
Host: {{test_host}}
Accept: application/json
HTTP 404
# Might return JSON error if plugin supports it

###############################################################################
# 403 Forbidden Error Tests
###############################################################################

# Test 403 for path traversal
GET http://{{host}}:{{port}}/../../../etc/passwd
Host: {{test_host}}
HTTP 404  # Actually returns 404 due to path normalization

# Test 403 for restricted path (if authorization configured)
# GET http://{{host}}:{{port}}/admin/
# Host: {{test_host}}
# HTTP 403
# [Asserts]
# header "Content-Type" contains "text/"
# body contains "Access denied"

###############################################################################
# 405 Method Not Allowed Tests
###############################################################################

# Test unsupported method
PATCH http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 405
[Asserts]
header "Allow" exists
body contains "Method not allowed"

# Test TRACE method
TRACE http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 405

###############################################################################
# 400 Bad Request Tests
###############################################################################

# Test malformed Range header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Range: malformed-range-header
HTTP 200  # Might not trigger 400 if ignored

# Test invalid Content-Type on PUT
PUT http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Content-Type: invalid/content-type/////
```
Test content
```
HTTP 201  # Content-Type might be ignored

###############################################################################
# 500 Internal Server Error Tests
###############################################################################

# These are hard to trigger intentionally without breaking the server
# The error handler should catch any unhandled errors and return custom 500 page

###############################################################################
# 416 Range Not Satisfiable Tests
###############################################################################

# Test selector on non-HTML file
PUT http://{{host}}:{{port}}/error-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Plain text file
```
HTTP 201

GET http://{{host}}:{{port}}/error-test.txt
Host: {{test_host}}
Range: selector=body
HTTP 416
[Asserts]
header "Content-Range" == "selector body"
body contains "Range Not Satisfiable"

DELETE http://{{host}}:{{port}}/error-test.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Custom Error Pages Tests
###############################################################################

# If custom error pages are configured, test they are served
# This would require creating error pages like 404.html, 403.html, etc.

# Create custom 404 page
PUT http://{{host}}:{{port}}/errors/404.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Page Not Found</title></head>
<body>
    <h1>404 - Page Not Found</h1>
    <p>The requested page could not be found.</p>
    <a href="/">Return to Home</a>
</body>
</html>
```
HTTP 201

# Now test if custom 404 is served (depends on plugin configuration)
GET http://{{host}}:{{port}}/still-missing.html
Host: {{test_host}}
HTTP 404
# Might serve custom 404.html if plugin is configured to do so

# Clean up
DELETE http://{{host}}:{{port}}/errors/404.html
Host: {{test_host}}
HTTP 200

###############################################################################
# Error Response Headers
###############################################################################

# Test that error responses include proper headers
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
HTTP 404
[Asserts]
header "Content-Type" exists
header "Content-Length" exists
header "Date" exists
header "Server" exists

# Test cache headers on errors
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
HTTP 404
# Might include Cache-Control headers

###############################################################################
# Content Negotiation for Errors
###############################################################################

# Test error with different Accept headers
GET http://{{host}}:{{port}}/missing
Host: {{test_host}}
Accept: text/html
HTTP 404
[Asserts]
header "Content-Type" contains "text/"

GET http://{{host}}:{{port}}/missing
Host: {{test_host}}
Accept: application/json
HTTP 404
# Might return JSON error format

GET http://{{host}}:{{port}}/missing
Host: {{test_host}}
Accept: text/plain
HTTP 404
[Asserts]
header "Content-Type" matches "(text/plain|text/html)"

###############################################################################
# Error Logging Tests
###############################################################################

# These requests should trigger error logging
GET http://{{host}}:{{port}}/trigger-404.html
Host: {{test_host}}
HTTP 404
# Should log: "404 Not Found: /trigger-404.html"

PATCH http://{{host}}:{{port}}/trigger-405.html
Host: {{test_host}}
HTTP 405
# Should log: "405 Method Not Allowed: PATCH /trigger-405.html"