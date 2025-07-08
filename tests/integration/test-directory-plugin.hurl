# Test directory plugin functionality

# Test 1: Access to non-admin area requires basic auth (configured globally)
GET http://{{host}}:{{port}}/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Test 2: Access to admin area with non-admin user should be denied
GET http://{{host}}:{{port}}/admin/
Authorization: Basic dXNlcjp1c2VyMTIz
HTTP 403

# Test 3: Access to admin area with admin user should be allowed
GET http://{{host}}:{{port}}/admin/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body contains "Admin Dashboard"

# Test 4: Admin area subdirectory should also require admin auth
GET http://{{host}}:{{port}}/admin/users.html
Authorization: Basic dXNlcjp1c2VyMTIz
HTTP 403

# Test 5: Non-admin paths starting with 'admin' should not be affected
# First create a file outside admin directory
PUT http://{{host}}:{{port}}/administrator.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
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

# Should be accessible with regular auth
GET http://{{host}}:{{port}}/administrator.html
Authorization: Basic dXNlcjp1c2VyMTIz
HTTP 200
[Asserts]
body contains "Administrator Page"

# Clean up
DELETE http://{{host}}:{{port}}/administrator.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200