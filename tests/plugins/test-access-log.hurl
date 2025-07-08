# Access Log Plugin Tests
# Tests Apache-style access logging functionality
# Note: These tests verify that requests are logged but cannot directly test log output

###############################################################################
# Basic Request Logging Tests
###############################################################################

# Test GET request logging
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /foo.html HTTP/1.1" 200 [size]

# Test GET with 404 logging
GET http://{{host}}:{{port}}/does-not-exist.html
Host: {{test_host}}
HTTP 404
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /does-not-exist.html HTTP/1.1" 404 [size]

# Test PUT request logging
PUT http://{{host}}:{{port}}/log-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Test content for logging
```
HTTP 201
# Access log should contain: 127.0.0.1 - - [timestamp] "PUT /log-test.txt HTTP/1.1" 201 [size]

# Test POST request logging
POST http://{{host}}:{{port}}/log-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Appended content
```
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "POST /log-test.txt HTTP/1.1" 200 [size]

# Test DELETE request logging
DELETE http://{{host}}:{{port}}/log-test.txt
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "DELETE /log-test.txt HTTP/1.1" 200 [size]

###############################################################################
# Different HTTP Methods Logging
###############################################################################

# Test HEAD request logging
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "HEAD /foo.html HTTP/1.1" 200 0

# Test OPTIONS request logging
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "OPTIONS /foo.html HTTP/1.1" 200 [size]

###############################################################################
# Different Status Codes Logging
###############################################################################

# Test 403 Forbidden logging
GET http://{{host}}:{{port}}/../etc/passwd
Host: {{test_host}}
HTTP 404
# Access log should log the 404 status

# Test 405 Method Not Allowed logging
PATCH http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 405
# Access log should contain: 127.0.0.1 - - [timestamp] "PATCH /foo.html HTTP/1.1" 405 [size]

# Test 416 Range Not Satisfiable logging
GET http://{{host}}:{{port}}/foo.txt
Host: {{test_host}}
Range: selector=body
HTTP 416
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /foo.txt HTTP/1.1" 416 [size]

###############################################################################
# Query Parameters Logging
###############################################################################

# Test request with query parameters
GET http://{{host}}:{{port}}/foo.html?param1=value1&param2=value2
Host: {{test_host}}
HTTP 200
# Access log should contain: "GET /foo.html?param1=value1&param2=value2 HTTP/1.1"

# Test request with encoded characters
GET http://{{host}}:{{port}}/foo.html?search=hello%20world
Host: {{test_host}}
HTTP 200
# Access log should contain: "GET /foo.html?search=hello%20world HTTP/1.1"

###############################################################################
# User Agent Logging
###############################################################################

# Test with custom User-Agent
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
User-Agent: TestBot/1.0
HTTP 200
# Access log should contain the User-Agent string

# Test without User-Agent
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain "-" for missing User-Agent

###############################################################################
# Referer Logging
###############################################################################

# Test with Referer header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Referer: http://example.com/page.html
HTTP 200
# Access log should contain the Referer

# Test without Referer
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain "-" for missing Referer

###############################################################################
# Authentication Username Logging
###############################################################################

# Test authenticated request logging (if auth is configured)
# GET http://{{host}}:{{port}}/foo.html
# Host: {{test_host}}
# Authorization: Basic YWRtaW46YWRtaW4xMjM=
# HTTP 200
# Access log should contain: 127.0.0.1 - admin [timestamp] "GET /foo.html HTTP/1.1" 200 [size]

###############################################################################
# Large Request Logging
###############################################################################

# Test logging of request with large body
PUT http://{{host}}:{{port}}/large-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
This is a larger file content that will be logged.
It contains multiple lines and should show the correct
response size in the access log after the request completes.
The access log plugin should handle requests of any size.
```
HTTP 201
# Access log should show correct response size

DELETE http://{{host}}:{{port}}/large-test.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Special Characters in URLs
###############################################################################

# Test URL with spaces (encoded)
GET http://{{host}}:{{port}}/file%20with%20spaces.html
Host: {{test_host}}
HTTP 404
# Access log should contain: "GET /file%20with%20spaces.html HTTP/1.1"

# Test URL with special characters
GET http://{{host}}:{{port}}/special-@-chars.html
Host: {{test_host}}
HTTP 404
# Access log should contain the exact URL requested

###############################################################################
# HTTP Version Logging
###############################################################################

# All requests should log HTTP/1.1 version
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain "HTTP/1.1"

###############################################################################
# Response Time Logging (if implemented)
###############################################################################

# Some access log implementations include response time
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log might contain response time in microseconds

###############################################################################
# Concurrent Request Logging
###############################################################################

# Multiple requests should each generate separate log entries
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200

GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Each request should have its own log entry with correct timestamp