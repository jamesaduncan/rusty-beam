# Rusty Beam HTTP Server Tests
# Test suite for the rusty-beam server using Hurl
# Run with: hurl tests.hurl --variable host=127.0.0.1 --variable port=3000

###############################################################################
# Basic HTTP Operations Tests
###############################################################################

# Test OPTIONS - Check server capabilities
OPTIONS http://{{host}}:{{port}}/
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "PUT"
header "Allow" contains "POST"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"
header "Accept-Ranges" == "selector"

# Test GET - Root index file
GET http://{{host}}:{{port}}/
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Example Page"
body contains "<h1>Example Page</h1>"

# Test GET - Specific HTML file
GET http://{{host}}:{{port}}/foo.html
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"
body contains "<h1>Hello, World</h1>"

# Test GET - Non-existent file
GET http://{{host}}:{{port}}/nonexistent.html
HTTP 404
[Asserts]
body == "File not found"

###############################################################################
# CSS Selector-based GET Operations
###############################################################################

# Test GET with CSS selector - Extract h1 element
GET http://{{host}}:{{port}}/foo.html
Range: selector=h1
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "<h1>Hello, World</h1>"

# Test GET with CSS selector - Extract specific list item
GET http://{{host}}:{{port}}/index.html
Range: selector=li:nth-child(2)
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Two"

# Test GET with CSS selector - Invalid selector
GET http://{{host}}:{{port}}/index.html
Range: selector=nonexistent
HTTP 404
[Asserts]
body == "No elements matched the selector"

# Test GET with selector on non-HTML file (should ignore selector)
GET http://{{host}}:{{port}}/nonexistent.txt
Range: selector=body
HTTP 404
[Asserts]
body == "File not found"

###############################################################################
# File Upload Tests (PUT)
###############################################################################

# Test PUT - Create new file
PUT http://{{host}}:{{port}}/test-upload.txt
Content-Type: text/plain
```
This is a test file uploaded via PUT.
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test PUT - Verify uploaded file
GET http://{{host}}:{{port}}/test-upload.txt
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"
body contains "This is a test file uploaded via PUT."

# Test PUT - Upload HTML file
PUT http://{{host}}:{{port}}/test.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <h1>Test Title</h1>
    <p>Test paragraph</p>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

###############################################################################
# CSS Selector-based PUT Operations
###############################################################################

# Test PUT with CSS selector - Replace h1 content
PUT http://{{host}}:{{port}}/test.html
Range: selector=h1
Content-Type: text/html
```
<h1>Updated Title</h1>
```
HTTP 200
[Asserts]
body contains "Updated Title"

# Test PUT with CSS selector - Replace paragraph content
PUT http://{{host}}:{{port}}/test.html
Range: selector=p
Content-Type: text/html
```
<p>Updated paragraph content</p>
```
HTTP 200
[Asserts]
body contains "Updated paragraph content"

# Test PUT with CSS selector - Invalid selector
PUT http://{{host}}:{{port}}/test.html
Range: selector=nonexistent
Content-Type: text/html
```
<div>New content</div>
```
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# File Append Tests (POST)
###############################################################################

# Test POST - Append to existing file
POST http://{{host}}:{{port}}/test-upload.txt
Content-Type: text/plain
```

Appended content via POST.
```
HTTP 200
[Asserts]
body == "Content appended successfully"

# Test POST - Verify appended content
GET http://{{host}}:{{port}}/test-upload.txt
HTTP 200
[Asserts]
body contains "This is a test file uploaded via PUT."
body contains "Appended content via POST."

# Test POST - Create new file via append
POST http://{{host}}:{{port}}/post-created.txt
Content-Type: text/plain
```
File created via POST.
```
HTTP 200
[Asserts]
body == "Content appended successfully"

###############################################################################
# CSS Selector-based POST Operations
###############################################################################

# Test POST with CSS selector - Append to h1 element
POST http://{{host}}:{{port}}/test.html
Range: selector=h1
Content-Type: text/html
```
<span> - Additional content</span>
```
HTTP 200
[Asserts]
body contains "Updated Title"
body contains "Additional content"

# Test POST with CSS selector - Append to body
POST http://{{host}}:{{port}}/test.html
Range: selector=body
Content-Type: text/html
```
<div>New div appended to body</div>
```
HTTP 200
[Asserts]
body contains "New div appended to body"

# Test POST with CSS selector - Invalid selector
POST http://{{host}}:{{port}}/test.html
Range: selector=nonexistent
Content-Type: text/html
```
<div>Should not work</div>
```
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# File Deletion Tests (DELETE)
###############################################################################

# Test DELETE - Remove uploaded file
DELETE http://{{host}}:{{port}}/test-upload.txt
HTTP 200
[Asserts]
body == "File deleted successfully"

# Test DELETE - Verify file is gone
GET http://{{host}}:{{port}}/test-upload.txt
HTTP 404
[Asserts]
body == "File not found"

# Test DELETE - Non-existent file
DELETE http://{{host}}:{{port}}/nonexistent.txt
HTTP 404
[Asserts]
body == "File not found"

###############################################################################
# CSS Selector-based DELETE Operations
###############################################################################

# Test DELETE with CSS selector - Remove specific element
DELETE http://{{host}}:{{port}}/test.html
Range: selector=div
HTTP 204

# Test DELETE with CSS selector - Invalid selector
DELETE http://{{host}}:{{port}}/test.html
Range: selector=nonexistent
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# Error Handling and Edge Cases
###############################################################################

# Test path traversal prevention
GET http://{{host}}:{{port}}/../README.md
HTTP 404
[Asserts]
body == "File not found"

# Test invalid method
PATCH http://{{host}}:{{port}}/test.html
HTTP 405
[Asserts]
body == "Method not allowed"

# Test CSS selector with non-HTML file
GET http://{{host}}:{{port}}/post-created.txt
Range: selector=body
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"
body contains "File created via POST."

# Test malformed selector header
GET http://{{host}}:{{port}}/test.html
Range: invalid-format
HTTP 200
[Asserts]
header "Content-Type" == "text/html"

# Test empty selector
GET http://{{host}}:{{port}}/test.html
Range: selector=
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# Content Type Detection Tests
###############################################################################

# Test HTML content type
GET http://{{host}}:{{port}}/test.html
HTTP 200
[Asserts]
header "Content-Type" == "text/html"

# Test plain text content type
GET http://{{host}}:{{port}}/post-created.txt
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"

# Test CSS file (upload and test)
PUT http://{{host}}:{{port}}/test.css
Content-Type: text/css
```
body { margin: 0; }
```
HTTP 201

GET http://{{host}}:{{port}}/test.css
HTTP 200
[Asserts]
header "Content-Type" == "text/css"
body contains "body { margin: 0; }"

# Test JavaScript file (upload and test)
PUT http://{{host}}:{{port}}/test.js
Content-Type: application/javascript
```
console.log("Hello from JavaScript");
```
HTTP 201

GET http://{{host}}:{{port}}/test.js
HTTP 200
[Asserts]
header "Content-Type" == "application/javascript"
body contains "console.log"

# Test JSON file (upload and test)
PUT http://{{host}}:{{port}}/test.json
Content-Type: application/json
```
{"message": "Hello World", "status": "success"}
```
HTTP 201

GET http://{{host}}:{{port}}/test.json
HTTP 200
[Asserts]
header "Content-Type" == "application/json"
body contains "Hello World"

###############################################################################
# Complex CSS Selector Tests
###############################################################################

# Upload a more complex HTML file for advanced selector testing
PUT http://{{host}}:{{port}}/complex.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Complex Test</title></head>
<body>
    <nav>
        <ul class="menu">
            <li><a href="/">Home</a></li>
            <li><a href="/about">About</a></li>
            <li><a href="/contact">Contact</a></li>
        </ul>
    </nav>
    <main>
        <article id="main-content">
            <h1>Main Article</h1>
            <p class="intro">Introduction paragraph</p>
            <p>Regular paragraph</p>
        </article>
        <aside>
            <h2>Sidebar</h2>
            <p>Sidebar content</p>
        </aside>
    </main>
</body>
</html>
```
HTTP 201

# Test class selector
GET http://{{host}}:{{port}}/complex.html
Range: selector=.intro
HTTP 200
[Asserts]
body contains "Introduction paragraph"

# Test ID selector
GET http://{{host}}:{{port}}/complex.html
Range: selector=%23main-content
HTTP 200
[Asserts]
body contains "Main Article"
body contains "Introduction paragraph"

# Test attribute selector
GET http://{{host}}:{{port}}/complex.html
Range: selector=a[href="/about"]
HTTP 200
[Asserts]
body contains "About"

# Test descendant selector
GET http://{{host}}:{{port}}/complex.html
Range: selector=nav li
HTTP 200
[Asserts]
body contains "Home"

# Test pseudo-selector
GET http://{{host}}:{{port}}/complex.html
Range: selector=li:first-child
HTTP 200
[Asserts]
body contains "Home"

# Test complex selector combination
GET http://{{host}}:{{port}}/complex.html
Range: selector=main article p.intro
HTTP 200
[Asserts]
body contains "Introduction paragraph"

###############################################################################
# Table Element PUT Tests (TD Bug Regression)
###############################################################################

# Create an HTML file with a table to test table element handling
PUT http://{{host}}:{{port}}/table-test.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Table Test</title></head>
<body>
    <table>
        <tr>
            <td id="cell1">Original Cell 1</td>
            <td id="cell2">Original Cell 2</td>
        </tr>
        <tr>
            <td class="data">Original Data 1</td>
            <td class="data">Original Data 2</td>
        </tr>
    </table>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test PUT with td element using ID selector
PUT http://{{host}}:{{port}}/table-test.html
Range: selector=%23cell1
Content-Type: text/html
```
<td id="cell1">Updated Cell 1 Content</td>
```
HTTP 200

# Verify the td was replaced correctly with full HTML tags preserved
GET http://{{host}}:{{port}}/table-test.html
HTTP 200
[Asserts]
body contains "<td id=\"cell1\">Updated Cell 1 Content</td>"

# Test PUT with td element using class selector
PUT http://{{host}}:{{port}}/table-test.html
Range: selector=.data
Content-Type: text/html
```
<td class="data">New Data Content</td>
```
HTTP 200

# Verify the first .data td was replaced with full HTML tags preserved
GET http://{{host}}:{{port}}/table-test.html
HTTP 200
[Asserts]
body contains "<td class=\"data\">New Data Content</td>"

# Test PUT with tr element replacement
PUT http://{{host}}:{{port}}/table-test.html
Range: selector=tr
Content-Type: text/html
```
<tr><td>New Row Cell 1</td><td>New Row Cell 2</td></tr>
```
HTTP 200

# Verify the tr was replaced correctly with full HTML structure preserved
GET http://{{host}}:{{port}}/table-test.html
HTTP 200
[Asserts]
body contains "<tr><td>New Row Cell 1</td><td>New Row Cell 2</td></tr>"

###############################################################################
# Host Header Routing Tests
###############################################################################

# Create test files for different hosts
PUT http://{{host}}:{{port}}/test.html
Host: localhost
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Localhost Test</title></head>
<body>
    <h1>This is localhost content</h1>
    <p>Served from ./localhost directory</p>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

PUT http://{{host}}:{{port}}/test.html
Host: example.com
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Example.com Test</title></head>
<body>
    <h1>This is example.com content</h1>
    <p>Served from ./example-com directory</p>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test retrieval with Host headers - should get different content based on Host
GET http://{{host}}:{{port}}/test.html
Host: localhost
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "This is localhost content"
body contains "Served from ./localhost directory"

GET http://{{host}}:{{port}}/test.html
Host: example.com
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "This is example.com content"
body contains "Served from ./example-com directory"

# Test that requests without Host header or unknown hosts fall back to default server_root
GET http://{{host}}:{{port}}/foo.html
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"

GET http://{{host}}:{{port}}/foo.html
Host: unknown-host.com
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"

# Test CSS selector operations work with Host headers
PUT http://{{host}}:{{port}}/test.html
Host: localhost
Range: selector=h1
Content-Type: text/html
```
<h1>Updated localhost title</h1>
```
HTTP 200

# Verify the update was applied to the correct host
GET http://{{host}}:{{port}}/test.html
Host: localhost
HTTP 200
[Asserts]
body contains "Updated localhost title"

# Verify the other host was not affected
GET http://{{host}}:{{port}}/test.html
Host: example.com
HTTP 200
[Asserts]
body contains "This is example.com content"
body not contains "Updated localhost title"

# Clean up host-specific test files
DELETE http://{{host}}:{{port}}/test.html
Host: localhost
HTTP 200

DELETE http://{{host}}:{{port}}/test.html
Host: example.com
HTTP 200

###############################################################################
# Cleanup
###############################################################################

# Clean up test files
DELETE http://{{host}}:{{port}}/test.html
HTTP 200

DELETE http://{{host}}:{{port}}/test.css
HTTP 200

DELETE http://{{host}}:{{port}}/test.js
HTTP 200

DELETE http://{{host}}:{{port}}/test.json
HTTP 200

DELETE http://{{host}}:{{port}}/complex.html
HTTP 200

DELETE http://{{host}}:{{port}}/table-test.html
HTTP 200

DELETE http://{{host}}:{{port}}/post-created.txt
HTTP 200