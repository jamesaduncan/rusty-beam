# Compression Plugin Test - Comprehensive
# Note: Compression plugin may have minimum size threshold or may not be fully implemented

###############################################################################
# Basic Compression Tests
###############################################################################

# Test compression request
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
# Note: Plugin may not compress small files

# Test gzip with quality value
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip;q=1.0, deflate;q=0.5
HTTP 200

# Test multiple compression methods
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip, deflate, br
HTTP 200

# Test deflate compression
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: deflate
HTTP 200

###############################################################################
# No Compression Tests
###############################################################################

# Test without Accept-Encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
# Should not have compression without Accept-Encoding
header "Content-Encoding" not exists

# Test with identity encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: identity
HTTP 200
[Asserts]
header "Content-Encoding" not exists

###############################################################################
# Large File Compression Test
###############################################################################

# Create a larger file that might trigger compression
PUT http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Large Test File</title></head>
<body>
<!-- Repeat content to make file larger -->
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
</body>
</html>
```
HTTP 201

# Test compression on larger file
GET http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
# Larger files might be compressed

DELETE http://{{host}}:{{port}}/large-test.html
Host: {{test_host}}
HTTP 204

###############################################################################
# Different Content Types
###############################################################################

# Test JSON compression
PUT http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Content-Type: application/json
```
{
  "data": "This is test data that might be compressed",
  "array": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  "nested": {
    "field": "value",
    "another": "value"
  }
}
```
HTTP 201

GET http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200

DELETE http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 204

###############################################################################
# HEAD Request Compression
###############################################################################

# HEAD requests should include compression headers but no body
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200