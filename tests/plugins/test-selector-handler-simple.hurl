# Selector Handler Plugin Test - Simplified for isolated environment

# Test GET with ID selector
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Range: selector=\#title
HTTP 206
[Asserts]
header "Content-Range" == "selector #title"
body == "<h1 id=\"title\">Test Page</h1>"

# Test GET with class selector
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Range: selector=.content
HTTP 206
[Asserts]
body contains "This is a test page"

# Update HTML file for testing (file already exists from template)
PUT http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
    <h1 id="header">Original Header</h1>
    <p>Content</p>
</body>
</html>
```
HTTP 200

# Test creating a new file (should return 201)
PUT http://{{host}}:{{port}}/test-created-file-abc123.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
    <h1>New Test File</h1>
    <p>This file was created by the test</p>
</body>
</html>
```
HTTP 201

# Verify the file was created by reading it
GET http://{{host}}:{{port}}/test-created-file-abc123.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "New Test File"

# Clean up - delete the file we just created
DELETE http://{{host}}:{{port}}/test-created-file-abc123.html
Host: {{test_host}}
HTTP 204

# Test PUT with selector to replace element
PUT http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: selector=\#header
Content-Type: text/html
```
<h1 id="header">Updated Header</h1>
```
HTTP 206

# Verify update
GET http://{{host}}:{{port}}/test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Updated Header"

# Test selector on non-HTML file
PUT http://{{host}}:{{port}}/plain.txt
Host: {{test_host}}
Content-Type: text/plain
```
Plain text content
```
HTTP 201

GET http://{{host}}:{{port}}/plain.txt
Host: {{test_host}}
Range: selector=body
HTTP 416

# Cleanup
DELETE http://{{host}}:{{port}}/test.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/plain.txt
Host: {{test_host}}
HTTP 200