# CORS Plugin Test - Simplified for isolated environment

# Test simple CORS request
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" == "*"

# Test CORS with different origin
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://another-site.com
HTTP 200
[Asserts]
header "Access-Control-Allow-Origin" == "*"