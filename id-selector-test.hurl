# Test PUT with ID selector

# Create HTML file with ID
PUT http://127.0.0.1:3000/id-test.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>ID Test</title></head>
<body>
    <h1 id="title">Original Title</h1>
    <p>Some content</p>
</body>
</html>
```
HTTP 201

# Test PUT with ID selector
PUT http://127.0.0.1:3000/id-test.html
Range: selector=%23title
Content-Type: text/html
```
<h1 id="new-title">New Title</h1>
```
HTTP 200

# Check result
GET http://127.0.0.1:3000/id-test.html
HTTP 200
[Asserts]
body contains "New Title"
body contains "id=\"new-title\""

# Clean up
DELETE http://127.0.0.1:3000/id-test.html
HTTP 200