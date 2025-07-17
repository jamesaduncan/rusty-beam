# JavaScript Engine Plugin Tests (ES6 Modules)

# Test 1: Simple JavaScript handler
GET http://{{host}}:{{port}}/hello
[Asserts]
status == 200
header "Content-Type" == "text/plain"
header "X-Powered-By" == "Rusty-Beam-JavaScript"
body contains "Hello from JavaScript!"
body contains "/hello"

# Test 2: API users endpoint
GET http://{{host}}:{{port}}/api/users
[Asserts]
status == 200
header "Content-Type" == "application/json"
jsonpath "$" count == 3
jsonpath "$[0].name" == "Alice"
jsonpath "$[1].name" == "Bob"
jsonpath "$[2].name" == "Charlie"

# Test 3: API status endpoint
GET http://{{host}}:{{port}}/api/status
[Asserts]
status == 200
header "Content-Type" == "application/json"
jsonpath "$.status" == "ok"
jsonpath "$.version" == "1.0.0"
jsonpath "$.timestamp" exists

# Test 4: API echo endpoint
POST http://{{host}}:{{port}}/api/echo
Content-Type: application/json
X-Test-Header: test-value
{"test": "data", "number": 42}
[Asserts]
status == 200
header "Content-Type" == "application/json"
jsonpath "$.method" == "POST"
jsonpath "$.path" == "/api/echo"
jsonpath "$.headers['content-type']" == "application/json"
jsonpath "$.headers['x-test-header']" == "test-value"
jsonpath "$.body" contains "test"

# Test 5: Async handler
GET http://{{host}}:{{port}}/async
[Asserts]
status == 200
header "Content-Type" == "application/json"
header "X-Async" == "true"
jsonpath "$.message" == "This response was generated asynchronously"
jsonpath "$.requestedPath" == "/async"
jsonpath "$.timestamp" exists

# Test 6: Index page
GET http://{{host}}:{{port}}/
[Asserts]
status == 200
header "Content-Type" == "text/html"
body contains "Welcome to Rusty Beam JavaScript Engine!"
body contains "Available Endpoints"

# Test 7: Non-existent API endpoint
GET http://{{host}}:{{port}}/api/nonexistent
[Asserts]
status == 404
header "Content-Type" == "application/json"
jsonpath "$.error" == "Endpoint not found"

# Test 8: JavaScript error handling
GET http://{{host}}:{{port}}/error-test
[Asserts]
# Should fall through to file handler since no JS file exists
status == 404