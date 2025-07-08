# Basic Auth Plugin Tests
# Tests HTTP Basic Authentication functionality

###############################################################################
# No Authentication Tests
###############################################################################

# Test GET without auth credentials (should fail)
GET http://{{host}}:{{port}}/auth-test.html
Host: {{test_host}}
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body contains "Authentication required"

# Test PUT without auth (should fail)
PUT http://{{host}}:{{port}}/auth-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Should not be created
```
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""

# Test POST without auth (should fail)
POST http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Should fail
```
HTTP 401

# Test DELETE without auth (should fail)
DELETE http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
HTTP 401

###############################################################################
# Valid Authentication Tests
###############################################################################

# Test GET with valid admin credentials
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"

# Test PUT with valid admin credentials
PUT http://{{host}}:{{port}}/auth-test.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```
Created with admin auth
```
HTTP 201
[Asserts]
body contains "File uploaded successfully"

# Test GET with valid user credentials (johndoe:doe123)
GET http://{{host}}:{{port}}/auth-test.txt
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
body contains "Created with admin auth"

# Test POST with admin credentials
POST http://{{host}}:{{port}}/auth-test.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```

Appended by admin
```
HTTP 200

# Test DELETE with admin credentials
DELETE http://{{host}}:{{port}}/auth-test.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body contains "File deleted successfully"

###############################################################################
# Invalid Authentication Tests
###############################################################################

# Test with wrong password
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46d3JvbmdwYXNz
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body contains "Authentication required"

# Test with non-existent user
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic bm9uZXhpc3RlbnQ6cGFzc3dvcmQ=
HTTP 401
[Asserts]
body contains "Authentication required"

# Test with malformed Basic auth header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic malformed
HTTP 401

# Test with wrong auth scheme
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Bearer YWRtaW46YWRtaW4xMjM=
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""

###############################################################################
# Case Sensitivity Tests
###############################################################################

# Test with lowercase "basic" (should fail - plugin is case-sensitive)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: basic YWRtaW46YWRtaW4xMjM=
HTTP 401

# Test with mixed case "BaSiC" (should fail - plugin is case-sensitive)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: BaSiC YWRtaW46YWRtaW4xMjM=
HTTP 401

###############################################################################
# Special Characters in Credentials
###############################################################################

# Create user with special characters (would need to be added to users.html)
# For now, test that special chars in wrong credentials are handled properly
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic dXNlcjpwYXNzd29yZCE=
HTTP 401

###############################################################################
# OPTIONS Method with Auth
###############################################################################

# OPTIONS should require auth too
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 401

# OPTIONS with valid auth
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
header "Allow" exists

###############################################################################
# HEAD Method with Auth
###############################################################################

# HEAD without auth
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 401

# HEAD with valid auth
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

###############################################################################
# Realm Configuration Test
###############################################################################

# Verify the realm is correctly set
GET http://{{host}}:{{port}}/test.html
Host: {{test_host}}
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""

###############################################################################
# Multiple Authorization Headers (Edge Case)
###############################################################################

# Test with duplicate Authorization headers (should use first one)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200

###############################################################################
# Empty Credentials
###############################################################################

# Test with empty username and password (encoded ":")
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic Og==
HTTP 401

# Test with only username (encoded "admin:")
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46
HTTP 401

# Test with only password (encoded ":admin123")
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic OmFkbWluMTIz
HTTP 401