# HTML Prettifier Plugin Tests

# Test 1: POST request should prettify HTML
POST http://{{host}}:{{port}}/test.html
Content-Type: text/html
```
<!DOCTYPE html><html><head><title>Test</title></head><body><h1>Hello</h1><p>World</p></body></html>
```
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
body contains "<!DOCTYPE html>"
body contains "  <head>"
body contains "    <title>Test</title>"
body contains "  <body>"
body contains "    <h1>Hello</h1>"
body contains "    <p>World</p>"

# Test 2: PUT request should prettify HTML
PUT http://{{host}}:{{port}}/test2.html
Content-Type: text/html
```
<html><body><div id="content"><h2>Title</h2><p>Some text</p></div></body></html>
```
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
body contains "<html>"
body contains "  <body>"
body contains "    <div id=\"content\">"
body contains "      <h2>Title</h2>"
body contains "      <p>Some text</p>"

# Test 3: DELETE request should prettify HTML response
DELETE http://{{host}}:{{port}}/test3.html
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
# The response should be prettified

# Test 4: GET request should NOT prettify HTML
GET http://{{host}}:{{port}}/unformatted.html
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
# Should remain unformatted
body matches "<html><body><p>This is unformatted</p></body></html>"

# Test 5: POST with non-HTML content type should NOT prettify
POST http://{{host}}:{{port}}/data.json
Content-Type: application/json
```
{"key":"value","nested":{"array":[1,2,3]}}
```
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
# JSON should remain unchanged
body == ```{"key":"value","nested":{"array":[1,2,3]}}```

# Test 6: POST with complex HTML including attributes
POST http://{{host}}:{{port}}/complex.html
Content-Type: text/html
```
<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Complex</title></head><body><div class="container" id="main"><p class="text" data-value="test">Text with <strong>bold</strong> and <em>italic</em></p><img src="image.jpg" alt="Test Image" /></div></body></html>
```
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
body contains "<!DOCTYPE html>"
body contains "<html lang=\"en\">"
body contains "  <head>"
body contains "    <meta charset=\"UTF-8\" />"
body contains "    <div class=\"container\" id=\"main\">"
body contains "      <p class=\"text\" data-value=\"test\">Text with <strong>bold</strong> and <em>italic</em></p>"
body contains "      <img src=\"image.jpg\" alt=\"Test Image\" />"

# Test 7: PUT with malformed HTML should still work
PUT http://{{host}}:{{port}}/malformed.html
Content-Type: text/html
```
<html><body><p>Unclosed paragraph<div>Another element</div></body></html>
```
HTTP 200
[Asserts]
header "Content-Type" contains "text/html"
# Should still attempt to prettify
body contains "  <body>"

# Test 8: POST empty HTML
POST http://{{host}}:{{port}}/empty.html
Content-Type: text/html
```

```
HTTP 200

# Test 9: DELETE with error response should NOT prettify
DELETE http://{{host}}:{{port}}/nonexistent.html
HTTP 404
[Asserts]
# Error pages should not be prettified by this plugin