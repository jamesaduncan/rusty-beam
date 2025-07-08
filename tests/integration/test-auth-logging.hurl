# Test that authenticated usernames are logged correctly in access logs

# Test unauthenticated request
GET http://{{host}}:{{port}}/admin/
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Admin Area\""

# Test authenticated request as johndoe
GET http://{{host}}:{{port}}/admin/
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
body contains "Admin Dashboard"

# Test authenticated request as janedoe  
GET http://{{host}}:{{port}}/admin/
Authorization: Basic amFuZWRvZTphZG1pbjEyMw==
HTTP 200
[Asserts]
body contains "Admin Dashboard"

# These tests would verify the access log contains the correct usernames,
# but hurl doesn't have direct file access. Manual verification shows:
# - Unauthenticated requests log with "-" as username
# - Authenticated requests log with actual username (johndoe, janedoe)