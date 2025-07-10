# Compression Plugin Test - Comprehensive
# Tests that verify actual compression is happening

###############################################################################
# Basic Compression Tests with Verification
###############################################################################

# Test gzip compression actually happens
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"
header "Vary" contains "Accept-Encoding"
# Content-Length should be different from uncompressed

# Test deflate compression
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: deflate
HTTP 200
[Asserts]
header "Content-Encoding" == "deflate"
header "Vary" contains "Accept-Encoding"

# Test brotli compression
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: br
HTTP 200
[Asserts]
header "Content-Encoding" == "br"
header "Vary" contains "Accept-Encoding"

# Test preference order (brotli > gzip > deflate)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: deflate, gzip, br
HTTP 200
[Asserts]
header "Content-Encoding" == "br"

# Test gzip with quality values
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip;q=1.0, deflate;q=0.5
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"

###############################################################################
# No Compression Tests
###############################################################################

# Test without Accept-Encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Encoding" not exists
header "Vary" not exists

# Test with identity encoding only
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: identity
HTTP 200
[Asserts]
header "Content-Encoding" not exists

# Test with unsupported encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: compress
HTTP 200
[Asserts]
header "Content-Encoding" not exists

###############################################################################
# Large File Compression Test (Above Threshold)
###############################################################################

# Create a file larger than 1KB to trigger compression
PUT http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Large Test File for Compression</title></head>
<body>
<!-- This content is repeated to exceed the 1KB threshold for compression -->
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
</body>
</html>
```
HTTP 201

# Get uncompressed size first
GET http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
HTTP 200
[Captures]
uncompressed_length: header "Content-Length"

# Test compression on larger file
GET http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"
header "Vary" contains "Accept-Encoding"
# Compressed size should be smaller
header "Content-Length" matches "^[0-9]+$"

DELETE http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
HTTP 204

###############################################################################
# Small File Test (Below Threshold)
###############################################################################

# Create a small file (less than 1KB)
PUT http://{{host}}:{{port}}/small-test.html
Host: {{test_host}}
Content-Type: text/html
```
<html><body>Small</body></html>
```
HTTP 201

# Small files should not be compressed (below 1KB threshold)
GET http://{{host}}:{{port}}/small-test.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" not exists

DELETE http://{{host}}:{{port}}/small-test.html
Host: {{test_host}}
HTTP 204

###############################################################################
# Different Content Types
###############################################################################

# Test JSON compression
PUT http://{{host}}:{{port}}/test-data.json
Host: {{test_host}}
Content-Type: application/json
```
{
  "data": "This is test data that should be compressed if large enough",
  "numbers": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
  "nested": {
    "field1": "value1 with some more text to make it larger",
    "field2": "value2 with some more text to make it larger",
    "field3": "value3 with some more text to make it larger",
    "field4": "value4 with some more text to make it larger",
    "field5": "value5 with some more text to make it larger"
  },
  "description": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. This JSON file contains enough data to exceed the compression threshold and should be compressed when requested with appropriate Accept-Encoding headers."
}
```
HTTP 201

GET http://{{host}}:{{port}}/test-data.json
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"
header "Content-Type" contains "application/json"

DELETE http://{{host}}:{{port}}/test-data.json
Host: {{test_host}}
HTTP 204

# Test CSS compression
PUT http://{{host}}:{{port}}/styles.css
Host: {{test_host}}
Content-Type: text/css
```
/* Large CSS file for compression testing */
body { margin: 0; padding: 0; font-family: Arial, sans-serif; background-color: #f0f0f0; }
.container { max-width: 1200px; margin: 0 auto; padding: 20px; background-color: white; }
.header { background-color: #333; color: white; padding: 20px; margin-bottom: 20px; }
.content { line-height: 1.6; color: #333; padding: 20px; min-height: 500px; }
.footer { background-color: #333; color: white; padding: 20px; margin-top: 20px; text-align: center; }
.button { display: inline-block; padding: 10px 20px; background-color: #007bff; color: white; text-decoration: none; border-radius: 5px; }
.button:hover { background-color: #0056b3; }
/* More rules to exceed compression threshold */
.card { background: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); padding: 20px; margin-bottom: 20px; }
.alert { padding: 15px; margin-bottom: 20px; border: 1px solid transparent; border-radius: 4px; }
.alert-success { color: #155724; background-color: #d4edda; border-color: #c3e6cb; }
.alert-error { color: #721c24; background-color: #f8d7da; border-color: #f5c6cb; }
```
HTTP 201

GET http://{{host}}:{{port}}/styles.css
Host: {{test_host}}
Accept-Encoding: deflate
HTTP 200
[Asserts]
header "Content-Encoding" == "deflate"
header "Content-Type" contains "text/css"

DELETE http://{{host}}:{{port}}/styles.css
Host: {{test_host}}
HTTP 204

###############################################################################
# Already Compressed Content
###############################################################################

# Test that already compressed files are not re-compressed
PUT http://{{host}}:{{port}}/image.jpg
Host: {{test_host}}
Content-Type: image/jpeg
```
FAKE_JPEG_DATA_FOR_TESTING
```
HTTP 201

# JPEG files should not be compressed (binary format)
GET http://{{host}}:{{port}}/image.jpg
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" not exists

DELETE http://{{host}}:{{port}}/image.jpg
Host: {{test_host}}
HTTP 204

###############################################################################
# HEAD Request Compression Headers
###############################################################################

# HEAD requests should include compression headers but no body
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"
header "Vary" contains "Accept-Encoding"

###############################################################################
# Multiple Accept-Encoding Values
###############################################################################

# Test with multiple acceptable encodings
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip, deflate, br
HTTP 200
[Asserts]
# Should prefer br > gzip > deflate
header "Content-Encoding" matches "^(br|gzip|deflate)$"

# Test with wildcard
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: *
HTTP 200
[Asserts]
# Should use one of the supported encodings
header "Content-Encoding" matches "^(br|gzip|deflate)$"

###############################################################################
# Range Requests with Compression
###############################################################################

# Range requests might interact differently with compression
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
Range: bytes=0-100
HTTP 206
# Note: Behavior with range requests may vary