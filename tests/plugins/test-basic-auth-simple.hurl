# Basic Auth Plugin Tests - Simplified Version
# Tests core authentication functionality

###############################################################################
# Authentication Required Tests
###############################################################################

# Request without credentials should fail
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""

# Request with valid credentials should succeed
# admin:admin123 encoded as Base64
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Request with invalid credentials should fail
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic aW52YWxpZDppbnZhbGlk
HTTP 401

# Another valid user (johndoe:doe123)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200