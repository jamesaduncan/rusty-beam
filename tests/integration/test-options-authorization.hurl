# Test OPTIONS method support in authorization plugin

# Test 1: Anonymous user OPTIONS request for entries
OPTIONS http://{{host}}:{{port}}/
Range: selector=#entries
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "POST"
header "Allow" contains "OPTIONS"
# Should NOT contain DELETE for anonymous users
header "Allow" not contains "DELETE"

# Test 2: Anonymous user OPTIONS request for specific entry
OPTIONS http://{{host}}:{{port}}/
Range: selector=#entries .entry
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "OPTIONS"
# Should NOT contain DELETE for anonymous users
header "Allow" not contains "DELETE"
header "Allow" not contains "POST"

# Test 3: Admin user OPTIONS request for entries
OPTIONS http://{{host}}:{{port}}/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=#entries .entry
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

# Test 4: OPTIONS request without selector
OPTIONS http://{{host}}:{{port}}/
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "OPTIONS"

# Test 5: OPTIONS request for non-existent path
OPTIONS http://{{host}}:{{port}}/nonexistent
HTTP 200
[Asserts]
header "Allow" contains "OPTIONS"
# May or may not contain other methods depending on wildcard rules