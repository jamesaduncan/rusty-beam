# File Handler Plugin Test - Simplified for isolated environment

# Test GET existing file
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello, World"

# Test GET non-existent file
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
HTTP 404

# Test PUT new file
PUT http://{{host}}:{{port}}/new-file.txt
Host: {{test_host}}
Content-Type: text/plain
```
This is a new file
```
HTTP 201

# Verify file was created
GET http://{{host}}:{{port}}/new-file.txt
Host: {{test_host}}
HTTP 200
[Asserts]
body contains "This is a new file"

# Test DELETE
DELETE http://{{host}}:{{port}}/new-file.txt
Host: {{test_host}}
HTTP 204

# Verify deletion
GET http://{{host}}:{{port}}/new-file.txt
Host: {{test_host}}
HTTP 404