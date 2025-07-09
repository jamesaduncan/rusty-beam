# Authorization Plugin Selector Tests - Demonstrating DOM-aware selector matching
# These tests will FAIL with the current implementation but will PASS after implementing proper selector support

###############################################################################
# Semantic Selector Matching Tests
###############################################################################

# Test 1: Basic element selector - should match any h1
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1
HTTP 206
[Asserts]
# Should return all h1 elements (allowed by h1 rule)
body contains "Public Header"

# Test 2: More specific selector should match if base element is allowed
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}  
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1.title
HTTP 206
[Asserts]
# Should work because h1.title is a subset of h1
body contains "Another Public Header"

# Test 3: Descendant selector should match if base element is allowed
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=  
Range: selector=article h1
HTTP 206
[Asserts]
# Should work because "article h1" selects h1 elements (which are allowed)
body contains "Article Header"

# Test 4: Pseudo-selector should match if base element is allowed
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1:first-child
HTTP 206
[Asserts]
# Should work because h1:first-child still selects h1 elements
body contains "Public Header"

# Test 5: Different element should be denied
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=div
HTTP 403
[Asserts]
# Should fail because div is not in allowed selectors
body contains "does not have permission"

# Test 6: Private class selector should be denied
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=.private
HTTP 403
[Asserts]
# Should fail due to explicit deny rule for .private
body contains "does not have permission"

# Test 7: Paragraph elements are allowed
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=p
HTTP 206
[Asserts]
body contains "public paragraph"

# Test 8: More specific paragraph selector should work
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=p.content
HTTP 206
[Asserts]
# p.content is a subset of p, so should be allowed
body contains "content paragraph"

# Test 9: Private content paragraph should be allowed because p is allowed
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=.private p
HTTP 206
[Asserts]
# Should succeed because we're accessing p elements (which are allowed)
# The .private deny rule only applies to elements with class="private", not their children
body contains "private content"

###############################################################################
# Admin Wildcard Selector Tests
###############################################################################

# Test 10: Admin can use any selector due to wildcard rule
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=.private
HTTP 206
[Asserts]
# Admin has selector="*" permission
body contains "private content"

# Test 11: Admin can use complex selectors
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=div.private p
HTTP 206
[Asserts]
body contains "private content"

###############################################################################
# Guestbook Specific Selector Permission
###############################################################################

# Test 12: Authenticated user can POST to guestbook with specific selector
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=ul.entry
Content-Type: text/html
```
<li>Test entry</li>
```
HTTP 206
[Asserts]
# Authenticated users can POST to guestbook with the allowed selector

# Test 13: Authenticated user cannot POST to guestbook with different selector
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1
Content-Type: text/html
```
<h1>Should fail</h1>
```
HTTP 403
[Asserts]
# Should fail because only ul.entry selector is allowed for POST
body contains "does not have permission"

# Test 14: More specific selector that still matches ul.entry should work
POST http://{{host}}:{{port}}/guestbook/
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=body ul.entry
Content-Type: text/html
```
<li>Another test entry</li>
```
HTTP 206
[Asserts]
# "body ul.entry" selects the same elements as "ul.entry"

###############################################################################
# Complex Selector Matching Tests
###############################################################################

# Test 15: Combined selectors are not yet supported
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1, p
HTTP 403
[Asserts]
# Combined selectors like "h1, p" are treated as a single string
# Future enhancement: parse and check each selector separately
body contains "does not have permission"

# Test 16: Combined selector with denied element should fail
GET http://{{host}}:{{port}}/selector-test.html
Host: {{test_host}}
Authorization: Basic am9obmRvZTpkb2UxMjM=
Range: selector=h1, .private
HTTP 403
[Asserts]
# Even though h1 is allowed, .private is denied
body contains "does not have permission"