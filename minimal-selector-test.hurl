# Minimal test for PUT with CSS selector

# Create HTML file
PUT http://127.0.0.1:3000/minimal.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <h1>Original</h1>
</body>
</html>
```
HTTP 201

# Test PUT with h1 selector
PUT http://127.0.0.1:3000/minimal.html
Range: selector=h1
Content-Type: text/html
```
<h1>Updated</h1>
```
HTTP 200

# Check result
GET http://127.0.0.1:3000/minimal.html
HTTP 200
[Asserts]
body contains "Updated"

# Clean up
DELETE http://127.0.0.1:3000/minimal.html
HTTP 200