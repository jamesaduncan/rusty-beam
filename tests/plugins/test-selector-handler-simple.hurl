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

# Create HTML file for testing
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
HTTP 201

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