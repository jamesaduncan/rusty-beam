# Security Headers Plugin Test - Comprehensive

###############################################################################
# X-XSS-Protection Tests
###############################################################################

# Test X-XSS-Protection header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-XSS-Protection" == "1; mode=block"

# Test for all HTML responses
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-XSS-Protection" exists

###############################################################################
# Content Type Options Tests
###############################################################################

# Test X-Content-Type-Options header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"

# Test on different content types
PUT http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Content-Type: application/json
```
{"test": true}
```
HTTP 201

GET http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"

DELETE http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200

###############################################################################
# Frame Options Tests
###############################################################################

# Test X-Frame-Options header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Frame-Options" == "SAMEORIGIN"

###############################################################################
# Content Security Policy Tests
###############################################################################

# Test CSP header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Security-Policy" exists
header "Content-Security-Policy" contains "default-src"

###############################################################################
# Referrer Policy Tests
###############################################################################

# Test Referrer-Policy header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Referrer-Policy" exists
header "Referrer-Policy" matches "(strict-origin-when-cross-origin|no-referrer|same-origin)"

###############################################################################
# Multiple Requests Test
###############################################################################

# Ensure headers are consistent across requests
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-XSS-Protection" exists
header "X-Content-Type-Options" exists
header "X-Frame-Options" exists

GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-XSS-Protection" exists
header "X-Content-Type-Options" exists
header "X-Frame-Options" exists

###############################################################################
# Non-HTML Resources
###############################################################################

# Create and test text file
PUT http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Plain text
```
HTTP 201

GET http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
HTTP 200
[Asserts]
# Security headers should still be present for all resources
header "X-Content-Type-Options" == "nosniff"
header "X-Frame-Options" exists

DELETE http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
HTTP 200