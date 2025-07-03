# Mixed Encryption Authentication Tests
# Test suite for validating mixed plaintext/bcrypt password support
# This test assumes the server is configured with users_mixed.html auth file
# Run with: hurl tests_auth_mixed.hurl --variable host=127.0.0.1 --variable port=3000

###############################################################################
# Mixed Encryption Types - Authentication Tests
###############################################################################

# Test bcrypt encrypted password (admin:admin123)
# Using bcrypt hash: $2b$12$7TzFqs.CAWoacMSlf7WOK.duP.vf1YwytKWKkDMTgLVgkmKjMZTW2
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello from localhost"

# Test plaintext password (johndoe:doe123)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello from localhost"

# Test second plaintext password (testuser:test123)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic dGVzdHVzZXI6dGVzdDEyMw==
HTTP 200
[Asserts]
header "Content-Type" == "text/html"
body contains "Hello from localhost"

###############################################################################
# Mixed Encryption - Invalid Credentials
###############################################################################

# Test wrong password for bcrypt user
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46d3JvbmdwYXNzd29yZA==
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body == "Authentication required"

# Test wrong password for plaintext user
GET http://{{host}}:{{port}}/index.html
Authorization: Basic am9obmRvZTp3cm9uZ3Bhc3N3b3Jk
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body == "Authentication required"

# Test non-existent user
GET http://{{host}}:{{port}}/index.html
Authorization: Basic bm9uZXhpc3RlbnQ6cGFzc3dvcmQ=
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body == "Authentication required"

###############################################################################
# Mixed Encryption - Role-Based Access Verification
###############################################################################

# Verify admin user has admin role (through successful access)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body contains "Hello from localhost"

# Verify johndoe user has user role (through successful access)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
body contains "Hello from localhost"

# Verify testuser has user role (through successful access)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic dGVzdHVzZXI6dGVzdDEyMw==
HTTP 200
[Asserts]
body contains "Hello from localhost"

###############################################################################
# Mixed Encryption - CRUD Operations
###############################################################################

# Test file creation with bcrypt user
PUT http://{{host}}:{{port}}/test-bcrypt-user.txt
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Content-Type: text/plain
```
Created by bcrypt user
```
HTTP 201

# Test file reading with plaintext user
GET http://{{host}}:{{port}}/test-bcrypt-user.txt
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
body == "Created by bcrypt user"

# Test file modification with plaintext user
PUT http://{{host}}:{{port}}/test-bcrypt-user.txt
Authorization: Basic dGVzdHVzZXI6dGVzdDEyMw==
Content-Type: text/plain
```
Modified by plaintext user
```
HTTP 200

# Verify modification with bcrypt user
GET http://{{host}}:{{port}}/test-bcrypt-user.txt
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body == "Modified by plaintext user"

# Test file deletion with plaintext user
DELETE http://{{host}}:{{port}}/test-bcrypt-user.txt
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200

###############################################################################
# Mixed Encryption - Stress Testing
###############################################################################

# Test rapid alternating requests between bcrypt and plaintext users
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

GET http://{{host}}:{{port}}/index.html
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200

GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

GET http://{{host}}:{{port}}/index.html
Authorization: Basic dGVzdHVzZXI6dGVzdDEyMw==
HTTP 200

GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200

###############################################################################
# Mixed Encryption - Edge Cases
###############################################################################

# Test that bcrypt password doesn't work as plaintext
# (This tests that the encryption field is actually being used)
GET http://{{host}}:{{port}}/index.html
Authorization: Basic YWRtaW46JDJiJDEyJDdUekZxcy5DQVdvYWNNU2xmN1dPSy5kdVAudmYxWXd5dEtXS2tETVRnTFZna21Lak1aVFcy
HTTP 401
[Asserts]
header "WWW-Authenticate" == "Basic realm=\"Rusty Beam\""
body == "Authentication required"