# Rusty Beam HTTP Server Tests
# Test suite for the rusty-beam server using Hurl
# Run with: hurl tests.hurl --variable host=127.0.0.1 --variable port=3000

###############################################################################
# Basic HTTP Operations Tests
###############################################################################

# Test OPTIONS - Check server capabilities
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
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
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Example Page"
body contains "<h1>Example Page</h1>"

# Test GET - Specific HTML file
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"
body contains "<h1>Hello, World</h1>"

# Test GET - Non-existent file
GET http://{{host}}:{{port}}/nonexistent.html
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

###############################################################################
# CSS Selector-based GET Operations
###############################################################################

# Test GET with CSS selector - Extract h1 element
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Range: selector=h1
HTTP 206
[Asserts]
header "Content-Type" == "text/html"
body contains "<h1>Hello, World</h1>"

# Test GET with CSS selector - Extract specific list item
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Range: selector=li:nth-child(2)
HTTP 206
[Asserts]
header "Content-Type" == "text/html"
body contains "Two"

# Test GET with CSS selector - Invalid selector
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Range: selector=nonexistent
HTTP 404
[Asserts]
body == "No elements matched the selector"

# Test GET with selector on non-HTML file (should ignore selector)
GET http://{{host}}:{{port}}/nonexistent.txt
Host: {{test_host}}
Range: selector=body
HTTP 404
[Asserts]
body == "File not found"

###############################################################################
# File Upload Tests (PUT)
###############################################################################

# Test PUT - Create new file
PUT http://{{host}}:{{port}}/test-upload.txt
Host: {{test_host}}
Content-Type: text/plain
```
This is a test file uploaded via PUT.
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test PUT - Verify uploaded file
GET http://{{host}}:{{port}}/test-upload.txt
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"
body contains "This is a test file uploaded via PUT."

# Test PUT - Upload HTML file
PUT http://{{host}}:{{port}}/test.html
Host: {{test_host}}
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
Host: {{test_host}}
Range: selector=h1
Content-Type: text/html
```
<h1>Updated Title</h1>
```
HTTP 206
[Asserts]
body contains "Updated Title"

# Test PUT with CSS selector - Replace paragraph content
PUT http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: selector=p
Content-Type: text/html
```
<p>Updated paragraph content</p>
```
HTTP 206
[Asserts]
body contains "Updated paragraph content"

# Test PUT with CSS selector - Invalid selector
PUT http://{{host}}:{{port}}/test.html
Host: {{test_host}}
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
Host: {{test_host}}
Content-Type: text/plain
```

Appended content via POST.
```
HTTP 200
[Asserts]
body == "Content appended successfully"

# Test POST - Verify appended content
GET http://{{host}}:{{port}}/test-upload.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "This is a test file uploaded via PUT."
body contains "Appended content via POST."

# Test POST - Create new file via append
POST http://{{host}}:{{port}}/post-created.txt
Host: {{test_host}}
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
Host: {{test_host}}
Range: selector=h1
Content-Type: text/html
```
<span> - Additional content</span>
```
HTTP 206
[Asserts]
body contains "Updated Title"
body contains "Additional content"

# Test POST with CSS selector - Append to body
POST http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: selector=body
Content-Type: text/html
```
<div>New div appended to body</div>
```
HTTP 206
[Asserts]
body contains "New div appended to body"

# Test POST with CSS selector - Invalid selector
POST http://{{host}}:{{port}}/test.html
Host: {{test_host}}
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
Host: {{test_host}}
HTTP 200
[Asserts]
body == "File deleted successfully"

# Test DELETE - Verify file is gone
GET http://{{host}}:{{port}}/test-upload.txt
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

# Test DELETE - Non-existent file
DELETE http://{{host}}:{{port}}/nonexistent.txt
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

###############################################################################
# CSS Selector-based DELETE Operations
###############################################################################

# Test DELETE with CSS selector - Remove specific element
DELETE http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: selector=div
HTTP 204

# Test DELETE with CSS selector - Invalid selector
DELETE http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: selector=nonexistent
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# HTTP Specification Compliance Tests
###############################################################################

# Test Content-Length header presence (RFC 7230 Section 3.3.2)
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Length" exists
header "Content-Length" matches "^[0-9]+$"

# Test Date header presence (RFC 7231 Section 7.1.1.2)
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Date" exists
header "Date" matches "^[A-Z][a-z]{2}, [0-9]{2} [A-Z][a-z]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT$"

# Test Server header presence (RFC 7231 Section 7.4.2)
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Server" exists

# Test HEAD method support (RFC 7231 Section 4.3.2)
HEAD http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Length" exists
header "Date" exists
header "Server" exists
header "Content-Type" exists

# Test case-insensitive header handling (RFC 7230 Section 3.2)
GET http://{{host}}:{{port}}/index.html
Host: LOCALHOST
HTTP 200

# Test Allow header includes HEAD (RFC 7231)
PATCH http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 405
[Asserts]
header "Allow" contains "HEAD"

###############################################################################
# HTTP Compliance Tests
###############################################################################

# Test PUT status codes - RFC 7231: 201 for new resources, 200 for updates
PUT http://{{host}}:{{port}}/put-status-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Initial content
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test PUT updating existing file - should return 200 OK, not 201
PUT http://{{host}}:{{port}}/put-status-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Updated content
```
HTTP 200
[Asserts]
body == "File uploaded successfully"

# Clean up test file
DELETE http://{{host}}:{{port}}/put-status-test.txt
Host: {{test_host}}
HTTP 200

###############################################################################
# Error Handling and Edge Cases
###############################################################################

# Test path traversal prevention
GET http://{{host}}:{{port}}/../README.md
Host: {{test_host}}
HTTP 404
[Asserts]
body == "File not found"

# Test invalid method - must include Allow header per RFC 7231
PATCH http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 405
[Asserts]
body == "Method not allowed"
header "Allow" exists
header "Allow" contains "GET"
header "Allow" contains "PUT"
header "Allow" contains "POST"
header "Allow" contains "DELETE"
header "Allow" contains "OPTIONS"

# Test CSS selector with non-HTML file (should return 416)
GET http://{{host}}:{{port}}/post-created.txt
Host: {{test_host}}
Range: selector=body
HTTP 416
[Asserts]
header "Content-Type" == "text/plain"
header "Content-Range" == "selector body"
body contains "Range Not Satisfiable"

# Test malformed selector header
GET http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: invalid-format
HTTP 200
[Asserts]
header "Content-Type" == "text/html"

# Test empty selector
GET http://{{host}}:{{port}}/test.html
Host: {{test_host}}
Range: selector=
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# Content Type Detection Tests
###############################################################################

# Test HTML content type
GET http://{{host}}:{{port}}/test.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"

# Test plain text content type
GET http://{{host}}:{{port}}/post-created.txt
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/plain"

# Test CSS file (upload and test)
PUT http://{{host}}:{{port}}/test.css
Host: {{test_host}}
Content-Type: text/css
```
body { margin: 0; }
```
HTTP 201

GET http://{{host}}:{{port}}/test.css
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/css"
body contains "body { margin: 0; }"

# Test JavaScript file (upload and test)
PUT http://{{host}}:{{port}}/test.js
Host: {{test_host}}
Content-Type: application/javascript
```
console.log("Hello from JavaScript");
```
HTTP 201

GET http://{{host}}:{{port}}/test.js
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/javascript"
body contains "console.log"

# Test JSON file (upload and test)
PUT http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Content-Type: application/json
```
{"message": "Hello World", "status": "success"}
```
HTTP 201

GET http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "application/json"
body contains "Hello World"

###############################################################################
# Complex CSS Selector Tests
###############################################################################

# Upload a more complex HTML file for advanced selector testing
PUT http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
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
Host: {{test_host}}
Range: selector=.intro
HTTP 206
[Asserts]
body contains "Introduction paragraph"

# Test ID selector
GET http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
Range: selector=%23main-content
HTTP 206
[Asserts]
body contains "Main Article"
body contains "Introduction paragraph"

# Test attribute selector
GET http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
Range: selector=a[href="/about"]
HTTP 206
[Asserts]
body contains "About"

# Test descendant selector
GET http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
Range: selector=nav li
HTTP 206
[Asserts]
body contains "Home"

# Test pseudo-selector
GET http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
Range: selector=li:first-child
HTTP 206
[Asserts]
body contains "Home"

# Test complex selector combination
GET http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
Range: selector=main article p.intro
HTTP 206
[Asserts]
body contains "Introduction paragraph"

###############################################################################
# Table Element PUT Tests (TD Bug Regression)
###############################################################################

# Create an HTML file with a table to test table element handling
PUT http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
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
Host: {{test_host}}
Range: selector=%23cell1
Content-Type: text/html
```
<td id="cell1">Updated Cell 1 Content</td>
```
HTTP 206

# Verify the td was replaced correctly with full HTML tags preserved
GET http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "<td id=\"cell1\">Updated Cell 1 Content</td>"

# Test PUT with td element using class selector
PUT http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
Range: selector=.data
Content-Type: text/html
```
<td class="data">New Data Content</td>
```
HTTP 206

# Verify the first .data td was replaced with full HTML tags preserved
GET http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "<td class=\"data\">New Data Content</td>"

# Test PUT with tr element replacement
PUT http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
Range: selector=tr
Content-Type: text/html
```
<tr><td>New Row Cell 1</td><td>New Row Cell 2</td></tr>
```
HTTP 206

# Verify the tr was replaced correctly with full HTML structure preserved
GET http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "<tr><td>New Row Cell 1</td><td>New Row Cell 2</td></tr>"

###############################################################################
# Host Header Routing Tests
###############################################################################

# Clean up any existing test.html files first
DELETE http://{{host}}:{{port}}/test.html
Host: localhost
HTTP *

DELETE http://{{host}}:{{port}}/test.html
Host: example.com
HTTP *

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
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"

GET http://{{host}}:{{port}}/foo.html
Host: unknown-host.com
HTTP 404
[Asserts]
header "Content-Type" == "text/plain"
body == "Host not found"

# Test CSS selector operations work with Host headers
PUT http://{{host}}:{{port}}/test.html
Host: localhost
Range: selector=h1
Content-Type: text/html
```
<h1>Updated localhost title</h1>
```
HTTP 206

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
Host: {{test_host}}
HTTP *

DELETE http://{{host}}:{{port}}/test.css
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.js
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/complex.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/table-test.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/post-created.txt
Host: {{test_host}}
HTTP 200
# Test that body tags are preserved when using PUT with CSS selectors

# First create a test file for body tag preservation
PUT http://{{host}}:{{port}}/body-tag-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Body Tag Test</title></head>
<body>
    <h1>Original Title</h1>
    <p>Original content</p>
</body>
</html>
```
HTTP *

# PUT request to update the body content using CSS selector
PUT http://{{host}}:{{port}}/body-tag-test.html
Host: {{test_host}}
Range: selector=body
Content-Type: text/html
```
<body>
    <h1>Updated Title</h1>
    <ul>
        <li>One</li>
        <li>Two</li>
        <li>Three</li>
    </ul>
</body>
```
HTTP 206

# Verify the body tags are preserved in the response
GET http://{{host}}:{{port}}/body-tag-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "<body>"
body contains "</body>"
body contains "<h1>Updated Title</h1>"

# Clean up
DELETE http://{{host}}:{{port}}/body-tag-test.html
Host: {{test_host}}
HTTP 200

# Test POST to append list items
PUT http://{{host}}:{{port}}/post-list-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>List Test</title></head>
<body>
    <h1>My List</h1>
    <ul>
        <li>Item 1</li>
        <li>Item 2</li>
    </ul>
</body>
</html>
```
HTTP *

# POST to ul selector to append item to list (correct way)
POST http://{{host}}:{{port}}/post-list-test.html
Host: {{test_host}}
Range: selector=ul
Content-Type: text/html
```
<li>Item 3</li>
```
HTTP 206

# Verify item was added to the list
GET http://{{host}}:{{port}}/post-list-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "<li>Item 1</li>"
body contains "<li>Item 2</li>"
body contains "<li>Item 3</li>"
xpath "//ul/li" count == 3

# Clean up
DELETE http://{{host}}:{{port}}/post-list-test.html
Host: {{test_host}}
HTTP 200

###############################################################################
# Root Path Selector Operations (Regression Tests)
# Tests for bug where "/" paths weren't properly mapped to index.html
###############################################################################

# Setup: Delete any existing index.html first
DELETE http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP *

# Create fresh index.html
PUT http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Root Test</title></head>
<body>
    <h1 id="title">Root Page</h1>
    <ul id="items">
        <li>First</li>
        <li>Second</li>
    </ul>
</body>
</html>
```
HTTP 201

# Test GET / with selector
GET http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=\#title
HTTP 206
[Asserts]
header "Content-Type" == "text/html"
body == "<h1 id=\"title\">Root Page</h1>"

# Test PUT / with selector
PUT http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=\#title
Content-Type: text/html
```
<h1 id="title">Updated Root Page</h1>
```
HTTP 206
[Asserts]
body == "<h1 id=\"title\">Updated Root Page</h1>"

# Test POST / with selector  
POST http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=\#items
Content-Type: text/html
```
<li>Third</li>
```
HTTP 206
[Asserts]
body contains "<li>Third</li>"

# Test DELETE / with selector
DELETE http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=\#items li:nth-child(2)
HTTP 204

# Verify deletion worked
GET http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=\#items
HTTP 206
[Asserts]
body contains "<li>First</li>"
body not contains "<li>Second</li>"
body contains "<li>Third</li>"

# Cleanup
DELETE http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
# Test 206 Partial Content responses with selectors
# These tests verify that selector-based responses return 206 status
# and include appropriate Content-Range headers

# Setup: Delete any existing file first
DELETE http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
HTTP *

# Create test HTML file
PUT http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Partial Content Test</title></head>
<body>
    <h1 id="title">Test Page</h1>
    <div id="content">
        <p class="intro">This is the introduction.</p>
        <ul id="items">
            <li>First item</li>
            <li>Second item</li>
            <li>Third item</li>
        </ul>
    </div>
    <footer id="footer">Page footer</footer>
</body>
</html>
```
HTTP 201

# Test GET with selector should return 206 Partial Content
GET http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Range: selector=\#title
HTTP 206
[Asserts]
header "Content-Type" == "text/html"
header "Content-Range" == "selector #title"
body == "<h1 id=\"title\">Test Page</h1>"

# Test GET with class selector should return 206
GET http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Range: selector=.intro
HTTP 206
[Asserts]
header "Content-Range" == "selector .intro"
body == "<p class=\"intro\">This is the introduction.</p>"

# Test GET with complex selector should return 206
GET http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Range: selector=\#items li:nth-child(2)
HTTP 206
[Asserts]
header "Content-Range" == "selector #items li:nth-child(2)"
body == "<li>Second item</li>"

# Test POST with HTML content and selector should return 206
POST http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Range: selector=\#footer
Content-Type: text/html
```
<p>Some HTML content</p>
```
HTTP 206
[Asserts]
header "Content-Range" == "selector #footer"
body == "<footer id=\"footer\">Page footer<p>Some HTML content</p>\n</footer>"

# Test PUT with selector should return 206 (for response body)
PUT http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Range: selector=\#content
Content-Type: text/html
<div id="content"><p>Updated content</p></div>
HTTP 206
[Asserts]
header "Content-Range" == "selector #content"
body == "<div id=\"content\"><p>Updated content</p></div>"

# Verify the update worked
GET http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
Range: selector=\#content
HTTP 206
[Asserts]
body == "<div id=\"content\"><p>Updated content</p></div>"

# Test that requests without selectors still return 200
GET http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Range" not exists
body contains "Test Page"
body contains "Updated content"

# Clean up
DELETE http://{{host}}:{{port}}/partial-test.html
Host: {{test_host}}
HTTP 200