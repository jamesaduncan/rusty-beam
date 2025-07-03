# Test PUT with CSS selector using replace_with_html

# First, create a test HTML file
PUT http://127.0.0.1:3000/selector-test.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Selector Test</title></head>
<body>
    <h1 id="title">Original Title</h1>
    <p class="content">Original paragraph</p>
    <div>Original div</div>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test PUT with selector - should replace the entire h1 element
PUT http://127.0.0.1:3000/selector-test.html
Range: selector=h1
Content-Type: text/html
```
<h1 id="title">Updated Title</h1>
```
HTTP 200

# Verify the h1 was replaced correctly (no nested h1 tags)
GET http://127.0.0.1:3000/selector-test.html
HTTP 200
[Asserts]
body contains "<h1 id=\"title\">Updated Title</h1>"
# Make sure we don't have nested h1 tags
body not contains "<h1><h1"

# Test PUT with selector on paragraph
PUT http://127.0.0.1:3000/selector-test.html
Range: selector=.content
Content-Type: text/html
```
<p class="content">Updated paragraph with more text</p>
```
HTTP 200

# Verify the paragraph was replaced
GET http://127.0.0.1:3000/selector-test.html
HTTP 200
[Asserts]
body contains "Updated paragraph with more text"

# Test PUT with ID selector
PUT http://127.0.0.1:3000/selector-test.html
Range: selector=#title
Content-Type: text/html
```
<h1 id="main-title">Main Title</h1>
```
HTTP 200

# Verify the element was replaced
GET http://127.0.0.1:3000/selector-test.html
HTTP 200
[Asserts]
body contains "Main Title"
body contains "id=\"main-title\""

# Clean up
DELETE http://127.0.0.1:3000/selector-test.html
HTTP 200
[Asserts]
body == "File deleted successfully"