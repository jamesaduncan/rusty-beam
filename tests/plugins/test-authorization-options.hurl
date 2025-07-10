# Authorization Plugin OPTIONS Method Tests
# Tests the OPTIONS method discovery functionality

###############################################################################
# Anonymous User OPTIONS Tests
###############################################################################

# Test 1: Anonymous OPTIONS for root path (no selector)
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200
[Asserts]
header "Allow" == "GET, OPTIONS"
header "Content-Type" == "text/plain"
header "Content-Length" == "0"

# Test 2: Anonymous OPTIONS for guestbook entries selector
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=#entries
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "OPTIONS"
header "Allow" not contains "DELETE"
header "Allow" not contains "PUT"

# Test 3: Anonymous OPTIONS for specific entry selector
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=#entries .entry
HTTP 200
[Asserts]
header "Allow" == "OPTIONS"  # Anonymous can't POST/DELETE to specific entries

# Test 4: Anonymous OPTIONS for non-existent path
OPTIONS http://{{host}}:{{port}}/nonexistent/path
Host: {{test_host}}
HTTP 200
[Asserts]
header "Allow" == "GET, OPTIONS"  # Wildcard rules apply

###############################################################################
# Regular User OPTIONS Tests (johndoe)
###############################################################################

# Test 5: Regular user OPTIONS for root
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Allow" == "GET, OPTIONS"

# Test 6: Regular user OPTIONS for test-resource path
OPTIONS http://{{host}}:{{port}}/test-resource/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "PUT"
header "Allow" contains "OPTIONS"
header "Allow" not contains "DELETE"

# Test 7: Regular user OPTIONS for user-specific path
OPTIONS http://{{host}}:{{port}}/users/johndoe/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "PUT"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

# Test 8: Regular user OPTIONS for another user's path
OPTIONS http://{{host}}:{{port}}/users/admin/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Allow" == "OPTIONS"  # Denied access to other users' paths

###############################################################################
# Admin User OPTIONS Tests
###############################################################################

# Test 9: Admin OPTIONS for root with selector
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=*
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "PUT"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

# Test 10: Admin OPTIONS for entries selector
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=#entries
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "PUT"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

# Test 11: Admin OPTIONS for specific entry
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=#entries .entry
HTTP 200
[Asserts]
header "Allow" contains "DELETE"  # Admin can delete specific entries

# Test 12: Admin OPTIONS for admin path
OPTIONS http://{{host}}:{{port}}/admin/
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "PUT"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

###############################################################################
# Edge Cases and Complex Scenarios
###############################################################################

# Test 13: OPTIONS with malformed selector
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=[invalid selector
HTTP 200
[Asserts]
header "Allow" == "GET, OPTIONS"  # Falls back to non-selector rules

# Test 14: OPTIONS for path with no matching rules
OPTIONS http://{{host}}:{{port}}/no-rules-defined-here
Host: {{test_host}}
HTTP 200
[Asserts]
header "Allow" == "GET, OPTIONS"  # Wildcard GET rule applies

# Test 15: OPTIONS for explicitly denied path (anonymous to admin area)
OPTIONS http://{{host}}:{{port}}/admin/secret.txt
Host: {{test_host}}
HTTP 200
[Asserts]
header "Allow" == "OPTIONS"  # Only OPTIONS allowed due to deny rule

###############################################################################
# Priority Testing - Role vs Username vs Wildcard
###############################################################################

# Test 16: User with editor role OPTIONS for content
OPTIONS http://{{host}}:{{port}}/content/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=  # johndoe has editor role
HTTP 200
[Asserts]
# Should show methods allowed by editor role if such rules exist
header "Allow" contains "GET"
header "Allow" contains "OPTIONS"

# Test 17: OPTIONS with conflicting allow/deny rules
# This tests that deny takes precedence at the same priority level
OPTIONS http://{{host}}:{{port}}/test-resource/sensitive
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Allow" contains "GET"  # Allowed by general rule
# Other methods depend on specific rule configuration