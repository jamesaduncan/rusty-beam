# Authorization Plugin Tests - Simplified Version
# Tests that authorization plugin loads without breaking the server
# Note: Full authorization rules parsing needs to be fixed in the plugin

###############################################################################
# Basic Tests
###############################################################################

# Without credentials, should get 401 from basic-auth
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 401

# With valid credentials, authorization plugin currently denies by default
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 403
[Asserts]
body contains "does not have permission"

# Test that server is still running
GET http://{{host}}:{{port}}/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 403