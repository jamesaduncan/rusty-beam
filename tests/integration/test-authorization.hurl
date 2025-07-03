# Test authorization system
# First, test without authentication - should fail
GET http://127.0.0.1:3000/test.html
Host: localhost
HTTP/1.1 401

# Test with authentication - should succeed for GET
GET http://127.0.0.1:3000/test.html
Host: localhost
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP/1.1 200
[Contains]
Test Page

# Test PUT without authentication - should fail
PUT http://127.0.0.1:3000/test.html
Host: localhost
Content-Type: text/html
```
<h1>Modified</h1>
```
HTTP/1.1 401

# Test PUT with authentication - should be forbidden per authorization rules
PUT http://127.0.0.1:3000/test.html
Host: localhost
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/html
```
<h1>Modified</h1>
```
HTTP/1.1 403