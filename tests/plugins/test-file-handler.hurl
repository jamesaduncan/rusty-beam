# File Handler Plugin Test - Comprehensive

###############################################################################
# GET - File Retrieval Tests
###############################################################################

# Test GET existing HTML file
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
header "Content-Length" exists
body contains "Hello, World"

# Test GET non-existent file
GET http://{{host}}:{{port}}/non-existent-file.html
Host: {{test_host}}
HTTP 404

# Test GET directory (should serve index.html)
GET http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Test Page"

# Test path traversal prevention
GET http://{{host}}:{{port}}/../../../etc/passwd
Host: {{test_host}}
HTTP 404

# Test GET with query parameters (should ignore them)
GET http://{{host}}:{{port}}/foo.html?param=value
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Hello, World"

###############################################################################
# PUT - File Creation and Update Tests
###############################################################################

# Test PUT new text file
PUT http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
This is a test file created by file-handler tests.
Line 2 of the test file.
```
HTTP 201
[Asserts]
body contains "File uploaded successfully"

# Verify file was created
GET http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"
body contains "This is a test file"
body contains "Line 2"

# Test PUT update existing file
PUT http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Updated content for the test file.
```
HTTP 200
[Asserts]
body contains "File uploaded successfully"

# Verify update
GET http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Updated content for the test file."

# Test PUT HTML file
PUT http://{{host}}:{{port}}/file-handler-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>File Handler Test</title></head>
<body>
    <h1>Test Page</h1>
    <p>Created by file-handler tests</p>
</body>
</html>
```
HTTP 201

# Test PUT to directory path (should create index.html)
PUT http://{{host}}:{{port}}/test-dir/
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html><body>Directory index</body></html>
```
HTTP 201

# Verify directory index was created
GET http://{{host}}:{{port}}/test-dir/
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Directory index"

###############################################################################
# POST - File Append Tests
###############################################################################

# Test POST to existing file
POST http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
Content-Type: text/plain
```

Appended line 1.
Appended line 2.
```
HTTP 200
[Asserts]
body contains "Content appended successfully"

# Verify append
GET http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Updated content"
body contains "Appended line 1"
body contains "Appended line 2"

# Test POST to create new file
POST http://{{host}}:{{port}}/new-post-file.txt
Host: {{test_host}}
Content-Type: text/plain
```
File created via POST
```
HTTP 200

###############################################################################
# DELETE - File Deletion Tests
###############################################################################

# Test DELETE existing file
DELETE http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "File deleted successfully"

# Verify deletion
GET http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
HTTP 404

# Test DELETE non-existent file
DELETE http://{{host}}:{{port}}/already-deleted.txt
Host: {{test_host}}
HTTP 404

# Delete other test files
DELETE http://{{host}}:{{port}}/file-handler-test.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/new-post-file.txt
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test-dir/index.html
Host: {{test_host}}
HTTP 200

###############################################################################
# HEAD - Metadata Retrieval Tests
###############################################################################

# Test HEAD on existing file
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
header "Content-Length" exists

# Test HEAD on non-existent file
HEAD http://{{host}}:{{port}}/does-not-exist.html
Host: {{test_host}}
HTTP 404

###############################################################################
# Content Type Detection Tests
###############################################################################

# Create files with different extensions
PUT http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Content-Type: application/json
```
{
    "test": true,
    "message": "File handler test"
}
```
HTTP 201

# Verify content types
GET http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"

# Cleanup
DELETE http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200

###############################################################################
# OPTIONS Method Test
###############################################################################

OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "PUT"
header "Allow" contains "POST"
header "Allow" contains "DELETE"
header "Allow" contains "HEAD"
header "Allow" contains "OPTIONS"