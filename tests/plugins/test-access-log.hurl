# Access Log Plugin Test - Comprehensive
# Note: Access log entries are written to stdout, so we verify behavior through requests

###############################################################################
# Basic Request Logging
###############################################################################

# Test GET request logging
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /foo.html HTTP/1.1" 200 [size]

# Test 404 logging
GET http://{{host}}:{{port}}/does-not-exist.html
Host: {{test_host}}
HTTP 404
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /does-not-exist.html HTTP/1.1" 404 [size]

###############################################################################
# Different HTTP Methods
###############################################################################

# Test PUT logging
PUT http://{{host}}:{{port}}/log-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Test content
```
HTTP 201
# Access log should contain: 127.0.0.1 - - [timestamp] "PUT /log-test.txt HTTP/1.1" 201 [size]

# Test POST logging
POST http://{{host}}:{{port}}/log-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Append content
```
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "POST /log-test.txt HTTP/1.1" 200 [size]

# Test DELETE logging
DELETE http://{{host}}:{{port}}/log-test.txt
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "DELETE /log-test.txt HTTP/1.1" 200 [size]

# Test HEAD logging
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "HEAD /foo.html HTTP/1.1" 200 [size]

# Test OPTIONS logging
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Access log should contain: 127.0.0.1 - - [timestamp] "OPTIONS /foo.html HTTP/1.1" 200 [size]

###############################################################################
# Special Cases
###############################################################################

# Test path traversal attempt logging
GET http://{{host}}:{{port}}/../../../etc/passwd
Host: {{test_host}}
HTTP 404
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /etc/passwd HTTP/1.1" 404 [size]

# Create text file for 416 test
PUT http://{{host}}:{{port}}/foo.txt
Host: {{test_host}}
Content-Type: text/plain
```
Sample text file
```
HTTP 201

# Test 416 Range Not Satisfiable logging
GET http://{{host}}:{{port}}/foo.txt
Host: {{test_host}}
Range: selector=body
HTTP 416
# Access log should contain: 127.0.0.1 - - [timestamp] "GET /foo.txt HTTP/1.1" 416 [size]

# Clean up
DELETE http://{{host}}:{{port}}/foo.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Query Parameters Logging
###############################################################################

# Test request with query parameters
GET http://{{host}}:{{port}}/foo.html?param1=value1&param2=value2
Host: {{test_host}}
HTTP 200
# Access log should contain: "GET /foo.html?param1=value1&param2=value2 HTTP/1.1"

###############################################################################
# Custom Headers
###############################################################################

# Test with X-Forwarded-For header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
X-Forwarded-For: 192.168.1.100
HTTP 200
# Access log might show forwarded IP: 192.168.1.100 - - [timestamp] "GET /foo.html HTTP/1.1" 200 [size]

# Test with User-Agent
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
User-Agent: TestBot/1.0
HTTP 200
# User agent is not typically in standard Apache log format, but might be in extended format