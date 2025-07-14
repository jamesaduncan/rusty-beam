# Test for .mjs file serving

# Test 1: GET .mjs file should have correct content type
GET http://{{host}}:{{port}}/test.mjs
HTTP 200
[Asserts]
header "Content-Type" == "application/javascript; charset=utf-8"
body contains "export"

# Test 2: HEAD .mjs file should have correct content type
HEAD http://{{host}}:{{port}}/test.mjs
HTTP 200
[Asserts]
header "Content-Type" == "application/javascript; charset=utf-8"

# Test 3: PUT .mjs file
PUT http://{{host}}:{{port}}/test2.mjs
Content-Type: application/javascript
```
export function hello() {
    return "Hello from module";
}
```
HTTP 201

# Test 4: GET the created .mjs file
GET http://{{host}}:{{port}}/test2.mjs
HTTP 200
[Asserts]
header "Content-Type" == "application/javascript; charset=utf-8"
body contains "export function hello"