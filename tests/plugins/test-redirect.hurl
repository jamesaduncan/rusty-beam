# Redirect Plugin Tests

# Test 1: Basic redirect from /old-page to /new-page with 301
GET http://{{host}}:{{port}}/old-page
HTTP 301
[Asserts]
header "Location" == "/new-page"
body contains "Redirecting to: /new-page"

# Test 2: Pattern redirect with capture group (/api/v1/* to /api/v2/*)
GET http://{{host}}:{{port}}/api/v1/users
HTTP 302
[Asserts]
header "Location" == "/api/v2/users"

# Test 3: Complex pattern redirect (blog URL structure)
GET http://{{host}}:{{port}}/blog/2024/03/my-post
HTTP 301
[Asserts]
header "Location" == "/posts/2024-03-my-post"

# Test 4: Non-matching URL should pass through
GET http://{{host}}:{{port}}/regular-page
HTTP 200

# Test 5: Multiple capture groups
GET http://{{host}}:{{port}}/api/v1/users/123/posts/456
HTTP 302
[Asserts]
header "Location" == "/api/v2/users/123/posts/456"

# Test 6: Root redirect
GET http://{{host}}:{{port}}/
HTTP 301
[Asserts]
header "Location" == "https://www.example.com/"

# Test 7: File extension removal (.php)
GET http://{{host}}:{{port}}/contact.php
HTTP 301
[Asserts]
header "Location" == "/contact"

# Test 8: 404 redirect
GET http://{{host}}:{{port}}/404
HTTP 302
[Asserts]
header "Location" == "/"

# Test 9: Ensure redirect preserves query parameters
GET http://{{host}}:{{port}}/old-page?foo=bar&baz=qux
HTTP 301
[Asserts]
header "Location" == "/new-page"

# Test 10: Complex regex with multiple parts
GET http://{{host}}:{{port}}/blog/2023/12/hello-world
HTTP 301
[Asserts]
header "Location" == "/posts/2023-12-hello-world"