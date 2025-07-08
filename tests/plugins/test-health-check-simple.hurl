# Health Check Plugin Test - Simplified for isolated environment

# Test health endpoint
GET http://{{host}}:{{port}}/health
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"
jsonpath "$.status" == "healthy"
jsonpath "$.timestamp" exists
jsonpath "$.checks" exists

# Test that normal files still work
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Test Page"