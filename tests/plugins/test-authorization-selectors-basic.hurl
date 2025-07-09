# Authorization Plugin Selector Tests - Basic String Matching
# These tests verify the basic selector authorization functionality

###############################################################################
# Basic Selector Authorization Tests
###############################################################################

# Test 1: Allowed selector - h1
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1
HTTP 206
[Asserts]
body contains "Public Header"

# Test 2: Allowed selector - p
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}  
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=p
HTTP 206
[Asserts]
body contains "public paragraph"

# Test 3: Denied selector - div (not in allowed list)
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=  
Range: selector=div
HTTP 403
[Asserts]
body contains "does not have permission"

# Test 4: Explicitly denied selector - .private
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=.private
HTTP 403
[Asserts]
body contains "does not have permission"

###############################################################################
# Admin Wildcard Selector Tests
###############################################################################

# Test 5: Admin can use any selector due to wildcard rule
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=.private
HTTP 206
[Asserts]
body contains "private content"

# Test 6: Admin can use div selector
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=div
HTTP 206

###############################################################################
# Guestbook Specific Selector Permission
###############################################################################

# Test 7: Anonymous can POST to guestbook with specific selector
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Range: selector=ul.entry
Content-Type: text/html
```
<li>Test entry</li>
```
HTTP 206

# Test 8: Anonymous cannot POST to guestbook with different selector
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Range: selector=h1
Content-Type: text/html
```
<h1>Should fail</h1>
```
HTTP 401

###############################################################################
# No Selector Tests (Path-only authorization)
###############################################################################

# Test 9: Regular GET without selector should work
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
HTTP 200
[Asserts]
body contains "Selector Authorization Test"

# Test 10: PUT without selector should be denied for regular user
PUT http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Content-Type: text/html
```
<p>Should fail</p>
```
HTTP 403