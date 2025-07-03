# Authentication System Integration Tests
# Test suite for the rusty-beam authentication plugin system
# Run with: hurl tests_auth.hurl --variable host=127.0.0.1 --variable port=3000

###############################################################################
# Basic Authentication Tests - No Credentials
###############################################################################

# Test accessing protected resource without credentials
GET http://{{host}}:{{port}}/index.html
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
header "Content-Type" == "text/plain"
body == "Authentication required"

# Test OPTIONS request (should not require authentication)
OPTIONS http://{{host}}:{{port}}/index.html
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "PUT"
header "Allow" contains "POST"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

###############################################################################
# Basic Authentication Tests - Invalid Credentials
###############################################################################

# Test with invalid username
GET http://{{host}}:{{port}}/index.html
Authorization: Basic bm9uZXhpc3RlbnQ6cGFzc3dvcmQ=
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body == "Authentication required"

# Test with invalid password for valid user
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46d3JvbmdwYXNzd29yZA==
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body == "Authentication required"

# Test with malformed Basic Auth header
GET http://{{host}}:{{port}}/index.html
Authorization: Basic invalidbase64!
HTTP 500
[Asserts]
header "Content-Type" == "text/plain"
body == "Authentication error"

# Test with non-Basic auth method
GET http://{{host}}:{{port}}/index.html
Authorization: Bearer some-token
HTTP 500
[Asserts]
header "Content-Type" == "text/plain"
body == "Authentication error"

###############################################################################
# Basic Authentication Tests - Valid Credentials (Plaintext)
###############################################################################

# Test with valid admin credentials (admin:admin123)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello from localhost"

# Test with valid user credentials (johndoe:doe123)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello from localhost"

# Test PUT operation with valid credentials
PUT http://{{host}}:{{port}}/test-auth.txt
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```
Test content for authentication
```
HTTP 201

# Verify the file was created
GET http://{{host}}:{{port}}/test-auth.txt
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body == "Test content for authentication"

# Test DELETE operation with valid credentials
DELETE http://{{host}}:{{port}}/test-auth.txt
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

###############################################################################
# CSS Selector Operations with Authentication
###############################################################################

# Test CSS selector GET with authentication
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=body
HTTP 200
[Asserts]
body contains "Hello from localhost"

# Test CSS selector PUT with authentication
PUT http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=h1
Content-Type: text/html
```
<h1>Updated with Auth</h1>
```
HTTP 200

# Verify the update worked
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=h1
HTTP 200
[Asserts]
body == "<h1>Updated with Auth</h1>"

# Restore original content
PUT http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=h1
Content-Type: text/html
```
<h1>Hello from localhost</h1>
```
HTTP 200

###############################################################################
# Host-Based Authentication Tests
###############################################################################

# Test that authentication is host-specific (localhost vs example.com)
# Note: This test assumes the server is configured for localhost authentication only

# Test with localhost host header (should require auth)
GET http://{{host}}:{{port}}/index.html
Host: localhost:{{port}}
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""

# Test with localhost host header and valid auth
GET http://{{host}}:{{port}}/index.html
Host: localhost:{{port}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body contains "Hello from localhost"

###############################################################################
# Error Handling Tests
###############################################################################

# Test authentication with empty credentials
GET http://{{host}}:{{port}}/index.html
Authorization: Basic 
HTTP 500
[Asserts]
body == "Authentication error"

# Test authentication with just username (no colon)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic dXNlcm5hbWU=
HTTP 500
[Asserts]
body == "Authentication error"

###############################################################################
# Performance and Edge Cases
###############################################################################

# Test multiple requests with same credentials (should work consistently)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Test case sensitivity of username
GET http://{{host}}:{{port}}/index.html
Authorization: Basic QURNSU46YWRtaW4xMjM=
HTTP 401
[Asserts]
body == "Authentication required"

# Test with very long invalid password
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46dmVyeWxvbmdpbnZhbGlkcGFzc3dvcmR0aGF0c2hvdWxkZmFpbA==
HTTP 401
[Asserts]
body == "Authentication required"