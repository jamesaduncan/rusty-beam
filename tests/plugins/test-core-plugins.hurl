# Core Plugin Tests - Simplified
# Tests basic functionality of essential plugins

###############################################################################
# File Handler Plugin Tests
###############################################################################

# Test basic file serving
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"

# Test 404
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
HTTP 404

# Test file creation
PUT http://{{host}}:{{port}}/test-file.txt
Host: {{test_host}}
Content-Type: text/plain
```
Test content
```
HTTP 201

# Test file retrieval
GET http://{{host}}:{{port}}/test-file.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Test content"

# Cleanup
DELETE http://{{host}}:{{port}}/test-file.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Selector Handler Plugin Tests
###############################################################################

# Delete if exists
DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP *

# Create HTML file
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
    <h1 id="title">Test Page</h1>
    <p class="content">Test paragraph</p>
</body>
</html>
```
HTTP 201

# Test ID selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#title
HTTP 206
[Asserts]
header "Content-Range" == "selector #title"
body contains "Test Page"

# Test class selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=.content
HTTP 206
[Asserts]
body contains "Test paragraph"

# Test element replacement
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=h1
Content-Type: text/html
```
<h1 id="title">Updated Title</h1>
```
HTTP 206

# Verify update
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Updated Title"

# Cleanup
DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP 200

###############################################################################
# Access Log Plugin Test (if configured)
###############################################################################

# Make request that should be logged
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Should generate access log entry

###############################################################################
# CORS Plugin Test (if configured)
###############################################################################

# Test CORS headers
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" exists

###############################################################################
# Health Check Plugin Test (if configured)
###############################################################################

# Test health endpoint
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"
jsonpath "$.status" == "healthy"

###############################################################################
# Compression Plugin Test (if configured)
###############################################################################

# Test gzip compression
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
# Content-Encoding header would exist if compression is working