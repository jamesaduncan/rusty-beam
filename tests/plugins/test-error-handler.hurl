# Error Handler Plugin Test - Comprehensive

###############################################################################
# 404 Not Found Error Tests
###############################################################################

# Test 404 error for missing file
GET http://{{host}}:{{port}}/non-existent-file.html
Host: {{test_host}}
HTTP 404
[Asserts]
header "Content-Type" contains "text/"
body contains "Not Found"
# Should return custom 404 page from error handler

# Test 404 for missing directory
GET http://{{host}}:{{port}}/missing-directory/
Host: {{test_host}}
HTTP 404
[Asserts]
body contains "Not Found"

# Test 404 with Accept header preferring HTML
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8
HTTP 404
[Asserts]
header "Content-Type" matches "(text/html|text/plain)"

###############################################################################
# 403 Forbidden Error Tests
###############################################################################

# Test 403 for path traversal (actually returns 404 due to path normalization)
GET http://{{host}}:{{port}}/../../../etc/passwd
Host: {{test_host}}
HTTP 404

###############################################################################
# 416 Range Not Satisfiable Tests
###############################################################################

# Create a text file
PUT http://{{host}}:{{port}}/error-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Plain text file
```
HTTP 201

# Test 416 with selector on non-HTML
GET http://{{host}}:{{port}}/error-test.txt
Host: {{test_host}}
Range: selector=body
HTTP 416
[Asserts]
body contains "Range Not Satisfiable"

DELETE http://{{host}}:{{port}}/error-test.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Error Response Format Tests
###############################################################################

# Test that error pages have proper structure
GET http://{{host}}:{{port}}/this-does-not-exist
Host: {{test_host}}
HTTP 404
[Asserts]
# Error handler should provide structured error page
body contains "404"
body contains "rusty-beam"  # Server identifier

# Test multiple sequential errors
GET http://{{host}}:{{port}}/error1.html
Host: {{test_host}}
HTTP 404

GET http://{{host}}:{{port}}/error2.html
Host: {{test_host}}
HTTP 404

GET http://{{host}}:{{port}}/error3.html
Host: {{test_host}}
HTTP 404