# Authorization Plugin Tests
# Tests role-based access control functionality
# Note: These tests assume basic-auth is also configured

###############################################################################
# Admin User Authorization Tests
###############################################################################

# Admin can GET any resource
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Admin can PUT
PUT http://{{host}}:{{port}}/admin-test.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```
Admin created this
```
HTTP 201

# Admin can POST
POST http://{{host}}:{{port}}/admin-test.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```

Admin appended this
```
HTTP 200

# Admin can DELETE
DELETE http://{{host}}:{{port}}/admin-test.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Admin can access /admin/* paths
PUT http://{{host}}:{{port}}/admin/config.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```
Admin only content
```
HTTP 201

###############################################################################
# Regular User Authorization Tests (johndoe)
###############################################################################

# Regular user can GET
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200

# Regular user cannot PUT (denied by default)
PUT http://{{host}}:{{port}}/user-test.txt
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Content-Type: text/plain
```
Should be denied
```
HTTP 403
[Asserts]
body contains "Access denied"

# Regular user cannot POST (denied by default)
POST http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Content-Type: text/html
```
<p>Should fail</p>
```
HTTP 403

# Regular user cannot DELETE
DELETE http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 403

# Regular user cannot access /admin/*
GET http://{{host}}:{{port}}/admin/config.txt
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 403

###############################################################################
# User-specific Path Authorization Tests
###############################################################################

# Create user directory
PUT http://{{host}}:{{port}}/users/johndoe/profile.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```
John Doe's profile
```
HTTP 201

# User can access their own directory
GET http://{{host}}:{{port}}/users/johndoe/profile.txt
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
body contains "John Doe's profile"

# User can modify their own files
PUT http://{{host}}:{{port}}/users/johndoe/data.txt
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Content-Type: text/plain
```
My personal data
```
HTTP 201

# User cannot access other users' directories
GET http://{{host}}:{{port}}/users/admin/profile.txt
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 403

# Admin can access any user directory
GET http://{{host}}:{{port}}/users/johndoe/data.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

###############################################################################
# Selector-based Authorization Tests
###############################################################################

# Admin can use selectors
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=h1
HTTP 206
[Asserts]
body contains "Hello, World"

# Admin can PUT with selectors
PUT http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=h1
Content-Type: text/html
```
<h1>Updated by Admin</h1>
```
HTTP 206

# Regular user gets read-only selector access
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1
HTTP 206

# Regular user cannot PUT with selectors
PUT http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1
Content-Type: text/html
```
<h1>Should fail</h1>
```
HTTP 403

###############################################################################
# Public Guestbook Selector Tests (Special Permission)
###############################################################################

# Create guestbook page
PUT http://{{host}}:{{port}}/guestbook/index.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<body>
    <h1>Guestbook</h1>
    <ul class="entry">
        <li>Welcome to our guestbook!</li>
    </ul>
</body>
</html>
```
HTTP 201

# Anonymous users can POST to specific selector
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Range: selector=ul.entry
Content-Type: text/html
```
<li>Anonymous entry</li>
```
HTTP 206

# But anonymous cannot POST to other selectors
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Range: selector=h1
Content-Type: text/html
```
<span>Should fail</span>
```
HTTP 401

###############################################################################
# Role-based Authorization Tests
###############################################################################

# Admin has 'administrators' role - can access admin areas
GET http://{{host}}:{{port}}/admin/
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 404  # 404 because file doesn't exist, not 403

# User without 'administrators' role cannot access
GET http://{{host}}:{{port}}/admin/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 403

###############################################################################
# Anonymous Access Tests
###############################################################################

# Anonymous can GET (per authorization rules)
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 401  # Basic auth blocks anonymous, but authorization would allow

# Anonymous cannot PUT
PUT http://{{host}}:{{port}}/anon-test.txt
Host: {{test_host}}
Content-Type: text/plain
```
Should fail at auth
```
HTTP 401

# Anonymous cannot DELETE
DELETE http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 401

###############################################################################
# Method-specific Authorization Tests
###############################################################################

# Test OPTIONS method authorization
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200

# Test HEAD method authorization
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200

###############################################################################
# Cleanup
###############################################################################

# Clean up admin files
DELETE http://{{host}}:{{port}}/admin/config.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Clean up user files
DELETE http://{{host}}:{{port}}/users/johndoe/profile.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

DELETE http://{{host}}:{{port}}/users/johndoe/data.txt
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Clean up guestbook
DELETE http://{{host}}:{{port}}/guestbook/index.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

# Restore original foo.html
PUT http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/html
```
<!DOCTYPE html>
<html lang="en">
<head>
    <title>Document</title>
</head>
<body>
    <h1>Hello, World</h1>
    <ul>
        <li>One</li>
        <li>Two</li>
        <li>Three</li>
    </ul>
</body>
</html>
```
HTTP 200