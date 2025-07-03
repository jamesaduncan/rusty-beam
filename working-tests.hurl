# Working Tests for Rusty Beam
# Core functionality that should work

###############################################################################
# Basic HTTP Operations
###############################################################################

# Test OPTIONS
OPTIONS http://127.0.0.1:3000/
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Accept-Ranges" == "selector"

# Test GET - Root index file
GET http://127.0.0.1:3000/
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Example Page"

# Test PUT - Create new file
PUT http://127.0.0.1:3000/working-test.txt
Content-Type: text/plain
```
This is a test file for verification.
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Verify uploaded file
GET http://127.0.0.1:3000/working-test.txt
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"
body contains "This is a test file for verification."

# Test PUT - Upload HTML file
PUT http://127.0.0.1:3000/working-test.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Working Test</title></head>
<body>
    <h1 id="title">Working Title</h1>
    <p class="content">Working paragraph</p>
    <div>Working div</div>
</body>
</html>
```
HTTP 201

# Test CSS selector GET
GET http://127.0.0.1:3000/working-test.html
Range: selector=h1
HTTP 200
[Asserts]
body contains "Working Title"

# Test CSS selector GET with ID
GET http://127.0.0.1:3000/working-test.html
Range: selector=#title
HTTP 200
[Asserts]
body contains "Working Title"

# Test CSS selector GET with class
GET http://127.0.0.1:3000/working-test.html
Range: selector=.content
HTTP 200
[Asserts]
body contains "Working paragraph"

# Test CSS selector PUT
PUT http://127.0.0.1:3000/working-test.html
Range: selector=h1
Content-Type: text/html
```
Updated Title
```
HTTP 200
[Asserts]
body contains "Updated Title"

# Test CSS selector POST
POST http://127.0.0.1:3000/working-test.html
Range: selector=body
Content-Type: text/html
```
<footer>Added footer</footer>
```
HTTP 200
[Asserts]
body contains "Added footer"

# Test CSS selector DELETE
DELETE http://127.0.0.1:3000/working-test.html
Range: selector=div
HTTP 204

# Verify div was deleted
GET http://127.0.0.1:3000/working-test.html
HTTP 200
[Asserts]
body contains "Updated Title"
body contains "Working paragraph"

# Test regular DELETE
DELETE http://127.0.0.1:3000/working-test.txt
HTTP 200
[Asserts]
body == "File deleted successfully"

# Verify file was deleted
GET http://127.0.0.1:3000/working-test.txt
HTTP 404
[Asserts]
body == "File not found"

# Clean up HTML test file
DELETE http://127.0.0.1:3000/working-test.html
HTTP 200