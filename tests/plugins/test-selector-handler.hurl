# Selector Handler Plugin Tests
# Tests CSS selector-based HTML manipulation functionality

###############################################################################
# Setup: Create test HTML files
###############################################################################

# Create a simple HTML file for basic selector tests
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Selector Test Page</title></head>
<body>
    <h1 id="main-title">Welcome</h1>
    <div class="container">
        <p class="intro">This is an introduction paragraph.</p>
        <ul id="list">
            <li>First item</li>
            <li>Second item</li>
            <li>Third item</li>
        </ul>
    </div>
    <footer>
        <p class="copyright">© 2025 Test Site</p>
    </footer>
</body>
</html>
```
HTTP 201

###############################################################################
# GET with Selectors - Basic Tests
###############################################################################

# Test ID selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#main-title
HTTP 206
[Asserts]
header "Content-Type" == "text/html"
header "Content-Range" == "selector #main-title"
body == "<h1 id=\"main-title\">Welcome</h1>"

# Test class selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=.intro
HTTP 206
[Asserts]
header "Content-Range" == "selector .intro"
body == "<p class=\"intro\">This is an introduction paragraph.</p>"

# Test element selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=footer
HTTP 206
[Asserts]
header "Content-Range" == "selector footer"
body contains "<footer>"
body contains "© 2025 Test Site"

# Test descendant selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=div p
HTTP 206
[Asserts]
body == "<p class=\"intro\">This is an introduction paragraph.</p>"

# Test child selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=ul > li:nth-child(2)
HTTP 206
[Asserts]
body == "<li>Second item</li>"

# Test non-existent selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#non-existent
HTTP 404
[Asserts]
body == "No elements matched the selector"

# Test empty selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# PUT with Selectors - Element Replacement
###############################################################################

# Replace h1 element
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=h1
Content-Type: text/html
```
<h1 id="main-title">Updated Welcome</h1>
```
HTTP 206
[Asserts]
header "Content-Range" == "selector h1"
body == "<h1 id=\"main-title\">Updated Welcome</h1>"

# Replace specific list item
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=li:first-child
Content-Type: text/html
```
<li>New first item</li>
```
HTTP 206
[Asserts]
body == "<li>New first item</li>"

# Replace with complex HTML
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=.container
Content-Type: text/html
```
<div class="container">
    <h2>New Section</h2>
    <p>Completely replaced content</p>
</div>
```
HTTP 206
[Asserts]
body contains "<h2>New Section</h2>"
body contains "Completely replaced content"

# Verify replacements persisted
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Updated Welcome"
body contains "New first item"
body contains "New Section"

###############################################################################
# POST with Selectors - Content Appending
###############################################################################

# Append to list
POST http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#list
Content-Type: text/html
```
<li>Fourth item</li>
<li>Fifth item</li>
```
HTTP 206
[Asserts]
header "Content-Range" == "selector #list"
body contains "Fourth item"
body contains "Fifth item"

# Append to body
POST http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=body
Content-Type: text/html
```
<script>console.log('Added via POST');</script>
```
HTTP 206
[Asserts]
body contains "<script>console.log('Added via POST');</script>"

###############################################################################
# DELETE with Selectors - Element Removal
###############################################################################

# Delete footer
DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=footer
HTTP 204

# Verify footer was deleted
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=footer
HTTP 404
[Asserts]
body == "No elements matched the selector"

# Delete specific list items
DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=li:nth-child(3)
HTTP 204

###############################################################################
# Table Element Handling (Special Cases)
###############################################################################

# Create table test file
PUT http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
    <table id="data-table">
        <thead>
            <tr><th>Name</th><th>Age</th></tr>
        </thead>
        <tbody>
            <tr><td>John</td><td>30</td></tr>
            <tr><td>Jane</td><td>25</td></tr>
        </tbody>
    </table>
</body>
</html>
```
HTTP 201

# Replace td element
PUT http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
Range: selector=td:first-child
Content-Type: text/html
```
<td>Johnny</td>
```
HTTP 206
[Asserts]
body == "<td>Johnny</td>"

# Replace tr element
PUT http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
Range: selector=tbody tr:last-child
Content-Type: text/html
```
<tr><td>Bob</td><td>35</td></tr>
```
HTTP 206
[Asserts]
body contains "<td>Bob</td>"
body contains "<td>35</td>"

# Replace tbody
PUT http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
Range: selector=tbody
Content-Type: text/html
```
<tbody>
    <tr><td>New</td><td>Data</td></tr>
</tbody>
```
HTTP 206
[Asserts]
body contains "<tbody>"
body contains "New"
body contains "Data"

###############################################################################
# Edge Cases and Error Handling
###############################################################################

# Selector on non-HTML file (should return 416)
PUT http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
Content-Type: text/plain
```
This is plain text
```
HTTP 201

GET http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
Range: selector=body
HTTP 416
[Asserts]
header "Content-Range" == "selector body"
body contains "Range Not Satisfiable"

# Selector on non-existent file
GET http://{{host}}:{{port}}/does-not-exist.html
Host: {{test_host}}
Range: selector=body
HTTP 404
[Asserts]
body == "File not found"

# Invalid selector syntax (should still process)
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=>>>invalid<<<
HTTP 404
[Asserts]
body == "No elements matched the selector"

# PUT with non-existent selector
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#does-not-exist
Content-Type: text/html
```
<div>Should fail</div>
```
HTTP 404
[Asserts]
body == "No elements matched the selector"

# POST with non-existent selector
POST http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=.missing-class
Content-Type: text/html
```
<p>Should fail</p>
```
HTTP 404
[Asserts]
body == "No elements matched the selector"

# DELETE with non-existent selector
DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#missing
HTTP 404
[Asserts]
body == "No elements matched the selector"

###############################################################################
# Complex Selectors
###############################################################################

# Attribute selector
GET http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
Range: selector=table[id="data-table"]
HTTP 206
[Asserts]
body contains "<table id=\"data-table\">"

# Multiple element selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=li
HTTP 206
[Asserts]
body contains "New first item"

# Pseudo-selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=li:last-child
HTTP 206
[Asserts]
body contains "Fifth item"

###############################################################################
# Root Path Handling
###############################################################################

# Create index.html
PUT http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
    <h1 id="home">Home Page</h1>
    <p class="welcome">Welcome to the site</p>
</body>
</html>
```
HTTP 201

# Test selector on root path
GET http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=\#home
HTTP 206
[Asserts]
header "Content-Range" == "selector #home"
body == "<h1 id=\"home\">Home Page</h1>"

# PUT to root with selector
PUT http://{{host}}:{{port}}/
Host: {{test_host}}
Range: selector=.welcome
Content-Type: text/html
```
<p class="welcome">Updated welcome message</p>
```
HTTP 206
[Asserts]
body contains "Updated welcome message"

###############################################################################
# Cleanup
###############################################################################

DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200