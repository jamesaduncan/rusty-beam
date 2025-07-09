# Selector Handler Plugin Test - Comprehensive

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
body contains "introduction paragraph"

# Test element selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=footer
HTTP 206
[Asserts]
body contains "© 2025 Test Site"

# Test nested selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=div.container p
HTTP 206
[Asserts]
body contains "introduction paragraph"

# Test list item selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=li:first-child
HTTP 206
[Asserts]
body == "<li>First item</li>"

###############################################################################
# GET with Selectors - Error Cases
###############################################################################

# Test non-existent selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=\#non-existent
HTTP 404

# Test empty selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=
HTTP 404

# Test invalid selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=>>>invalid<<<
HTTP 404

# Test selector on non-HTML file
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

# Selector on non-existent file
GET http://{{host}}:{{port}}/does-not-exist.html
Host: {{test_host}}
Range: selector=body
HTTP 404

###############################################################################
# PUT with Selectors - Element Replacement
###############################################################################

# Replace title element
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
body contains "<h1 id=\"main-title\">Updated Welcome</h1>"

# Replace specific list item
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Range: selector=li:first-child
Content-Type: text/html
```
<li>New first item</li>
```
HTTP 206

# Replace entire div
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

# Verify replacements persisted
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "Updated Welcome"
body contains "New Section"
body contains "Completely replaced content"

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
<td>Updated Name</td>
```
HTTP 206

# Get specific table row
GET http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
Range: selector=tr:nth-child(2)
HTTP 206
[Asserts]
body contains "<td>Jane</td>"

###############################################################################
# Cleanup
###############################################################################

DELETE http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
HTTP 204

DELETE http://{{host}}:{{port}}/table-selector-test.html
Host: {{test_host}}
HTTP 204

DELETE http://{{host}}:{{port}}/test.txt
Host: {{test_host}}
HTTP 204