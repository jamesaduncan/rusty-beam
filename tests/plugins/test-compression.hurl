# Compression Plugin Tests
# Tests gzip and deflate compression functionality

###############################################################################
# Gzip Compression Tests
###############################################################################

# Test gzip compression request
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"
header "Vary" contains "Accept-Encoding"
# Body should be gzip compressed

# Test gzip with quality value
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip;q=1.0, deflate;q=0.5
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"

# Test multiple compression methods
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip, deflate, br
HTTP 200
[Asserts]
header "Content-Encoding" matches "^(gzip|deflate)$"

###############################################################################
# Deflate Compression Tests
###############################################################################

# Test deflate compression
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: deflate
HTTP 200
[Asserts]
header "Content-Encoding" == "deflate"
header "Vary" contains "Accept-Encoding"

# Test deflate preferred over gzip
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip;q=0.5, deflate;q=1.0
HTTP 200
[Asserts]
header "Content-Encoding" == "deflate"

###############################################################################
# No Compression Tests
###############################################################################

# Test no compression when not requested
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Encoding" not exists
body contains "Hello, World"

# Test identity encoding (no compression)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: identity
HTTP 200
[Asserts]
header "Content-Encoding" not exists

# Test unsupported encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: br
HTTP 200
[Asserts]
header "Content-Encoding" not exists

###############################################################################
# Content Type Compression Tests
###############################################################################

# Test HTML compression
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
header "Content-Encoding" == "gzip"

# Test CSS compression
PUT http://{{host}}:{{port}}/test.css
Host: {{test_host}}
Content-Type: text/css
```
body { margin: 0; padding: 20px; font-family: Arial; }
```
HTTP 201

GET http://{{host}}:{{port}}/test.css
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Type" == "text/css"
header "Content-Encoding" == "gzip"

# Test JavaScript compression
PUT http://{{host}}:{{port}}/test.js
Host: {{test_host}}
Content-Type: application/javascript
```
function test() { console.log('Compression test'); }
```
HTTP 201

GET http://{{host}}:{{port}}/test.js
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Type" == "application/javascript"
header "Content-Encoding" == "gzip"

# Test JSON compression
PUT http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Content-Type: application/json
```
{"test": true, "compression": "enabled", "data": [1,2,3,4,5]}
```
HTTP 201

GET http://{{host}}:{{port}}/test.json
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Type" == "application/json"
header "Content-Encoding" == "gzip"

# Test that binary files might not be compressed
PUT http://{{host}}:{{port}}/test.bin
Host: {{test_host}}
Content-Type: application/octet-stream
```
Binary data here
```
HTTP 201

GET http://{{host}}:{{port}}/test.bin
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
# Binary files might not be compressed depending on implementation

###############################################################################
# Compression with Other Headers
###############################################################################

# Test compression with Range requests (should not compress)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
Range: bytes=0-10
HTTP 206
[Asserts]
header "Content-Encoding" not exists

# Test compression with HEAD requests
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"

###############################################################################
# Quality Values Tests
###############################################################################

# Test quality value 0 (don't use)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: gzip;q=0
HTTP 200
[Asserts]
header "Content-Encoding" not exists

# Test wildcard with quality
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: *;q=0.5, gzip;q=1.0
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"

###############################################################################
# Edge Cases
###############################################################################

# Test malformed Accept-Encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: ;;;invalid;;;
HTTP 200
[Asserts]
header "Content-Encoding" not exists

# Test empty Accept-Encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: 
HTTP 200
[Asserts]
header "Content-Encoding" not exists

# Test case insensitive encoding
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Accept-Encoding: GZIP
HTTP 200
[Asserts]
header "Content-Encoding" == "gzip"

###############################################################################
# Cleanup
###############################################################################

DELETE http://{{host}}:{{port}}/test.css
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.js
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.json
Host: {{test_host}}
HTTP 200

DELETE http://{{host}}:{{port}}/test.bin
Host: {{test_host}}
HTTP 200