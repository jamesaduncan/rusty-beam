# File Handler Plugin Tests
# Tests basic file serving and manipulation functionality

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
[Asserts]
body == "File not found"

# Test GET directory (should serve index.html)
GET http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Example Page"

# Test path traversal prevention
GET http://{{host}}:{{port}}/../../../etc/passwd
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

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
body == "File uploaded successfully"

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
body == "File uploaded successfully"

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
[Asserts]
body == "File uploaded successfully"

# Test PUT JSON file
PUT http://{{host}}:{{port}}/file-handler-test.json
Host: {{test_host}}
Content-Type: application/json
```
{
    "test": true,
    "message": "File handler test",
    "data": [1, 2, 3]
}
```
HTTP 201

# Test PUT CSS file
PUT http://{{host}}:{{port}}/file-handler-test.css
Host: {{test_host}}
Content-Type: text/css
```
body {
    margin: 0;
    padding: 0;
    font-family: Arial, sans-serif;
}
```
HTTP 201

# Test PUT JavaScript file
PUT http://{{host}}:{{port}}/file-handler-test.js
Host: {{test_host}}
Content-Type: application/javascript
```
function testFunction() {
    console.log('File handler test');
}
```
HTTP 201

# Test PUT with path traversal attempt
PUT http://{{host}}:{{port}}/../outside/bad.txt
Host: {{test_host}}
Content-Type: text/plain
```
Should not be created
```
HTTP 403
[Asserts]
body == "Access denied"

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
body == "Content appended successfully"

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
[Asserts]
body == "Content appended successfully"

# Verify POST-created file
GET http://{{host}}:{{port}}/new-post-file.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "File created via POST"

# Test POST to HTML file
POST http://{{host}}:{{port}}/file-handler-test.html
Host: {{test_host}}
Content-Type: text/html
```
    <div>Appended content</div>
</body>
</html>
```
HTTP 200

# Test POST with empty body
POST http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
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
body == "File deleted successfully"

# Verify deletion
GET http://{{host}}:{{port}}/file-handler-test.txt
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

# Test DELETE non-existent file
DELETE http://{{host}}:{{port}}/already-deleted.txt
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

# Test DELETE with path traversal
DELETE http://{{host}}:{{port}}/../../../etc/passwd
Host: {{test_host}}
HTTP 403
[Asserts]
body == "Access denied"

# Delete other test files
DELETE http://{{host}}:{{port}}/file-handler-test.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/file-handler-test.json
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/file-handler-test.css
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/file-handler-test.js
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

# Test HEAD on directory
HEAD http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"

###############################################################################
# Content Type Detection Tests
###############################################################################

# Create files with different extensions
PUT http://{{host}}:{{port}}/test.xml
Host: {{test_host}}
Content-Type: application/xml
```
<?xml version="1.0"?>
<root><item>Test</item></root>
```
HTTP 201

PUT http://{{host}}:{{port}}/test.svg
Host: {{test_host}}
Content-Type: image/svg+xml
```
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
    <circle cx="50" cy="50" r="40" fill="red"/>
</svg>
```
HTTP 201

PUT http://{{host}}:{{port}}/test.md
Host: {{test_host}}
Content-Type: text/markdown
```
# Markdown Test
This is a **test** file.
```
HTTP 201

# Verify content types
GET http://{{host}}:{{port}}/test.xml
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/xml"

GET http://{{host}}:{{port}}/test.svg
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "image/svg+xml"

GET http://{{host}}:{{port}}/test.md
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/markdown"

# Cleanup
DELETE http://{{host}}:{{port}}/test.xml
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.svg
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.md
Host: {{test_host}}
HTTP 200

###############################################################################
# Edge Cases and Special Characters
###############################################################################

# Test file with spaces in name
PUT http://{{host}}:{{port}}/file%20with%20spaces.txt
Host: {{test_host}}
Content-Type: text/plain
```
File with spaces in the name
```
HTTP 201

GET http://{{host}}:{{port}}/file%20with%20spaces.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "File with spaces in the name"

DELETE http://{{host}}:{{port}}/file%20with%20spaces.txt
Host: {{test_host}}
HTTP 200

# Test file with special characters (URL encoded)
PUT http://{{host}}:{{port}}/special-chars-%21%40%23.txt
Host: {{test_host}}
Content-Type: text/plain
```
Special characters test
```
HTTP 201

GET http://{{host}}:{{port}}/special-chars-%21%40%23.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Special characters test"

DELETE http://{{host}}:{{port}}/special-chars-%21%40%23.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Binary File Handling
###############################################################################

# Test small binary data (base64 encoded in test, but sent as binary)
PUT http://{{host}}:{{port}}/test.bin
Host: {{test_host}}
Content-Type: application/octet-stream
```
Binary test data here
```
HTTP 201

GET http://{{host}}:{{port}}/test.bin
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/octet-stream"

DELETE http://{{host}}:{{port}}/test.bin
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