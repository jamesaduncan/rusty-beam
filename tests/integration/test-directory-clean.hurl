# Test directory plugin functionality with clean config

# Test 1: Access to non-admin area should work without auth
GET http://{{host}}:{{port}}/
HTTP 200
[Asserts]
body contains "Example Title"

# Test 2: Access to admin area without auth should require it
GET http://{{host}}:{{port}}/admin/
HTTP 401
[Asserts]
header "WWW-Authenticate" contains "Admin Area"

# Test 3: Access to admin area with non-admin user should be denied  
GET http://{{host}}:{{port}}/admin/
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 403

# Test 4: Access to admin area with admin user should work
GET http://{{host}}:{{port}}/admin/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body contains "Admin Dashboard"

# Test 5: Non-admin paths should not require auth
GET http://{{host}}:{{port}}/index.html
HTTP 200

# Test 6: Path starting with 'admin' but not under /admin/ should not require auth
PUT http://{{host}}:{{port}}/administrator.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
<h1>Administrator Page</h1>
</body>
</html>
```
HTTP *

GET http://{{host}}:{{port}}/administrator.html
HTTP 200
[Asserts]
body contains "Administrator Page"

# Clean up
DELETE http://{{host}}:{{port}}/administrator.html
HTTP 200