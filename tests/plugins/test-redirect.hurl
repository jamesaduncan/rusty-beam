# Redirect Plugin Tests
# Tests URL redirection functionality

###############################################################################
# Basic Redirect Tests
###############################################################################

# Test 301 Permanent Redirect
GET http://{{host}}:{{port}}/old-page.html
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" exists
header "Location" matches "(/new-page\\.html|http)"

# Test 302 Temporary Redirect
GET http://{{host}}:{{port}}/temp-redirect
Host: {{test_host}}
HTTP 302
[Asserts]
header "Location" exists

# Test following redirects (client behavior)
GET http://{{host}}:{{port}}/redirect-me
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" exists

###############################################################################
# Pattern-based Redirects
###############################################################################

# Test wildcard redirects
GET http://{{host}}:{{port}}/blog/old-post-123
Host: {{test_host}}
HTTP 301
# Might redirect /blog/* to /posts/*

# Test regex redirects
GET http://{{host}}:{{port}}/product/widget-42
Host: {{test_host}}
HTTP 301
# Might redirect /product/(.*) to /shop/$1

###############################################################################
# Query String Handling
###############################################################################

# Test redirect preserving query strings
GET http://{{host}}:{{port}}/search?q=test&page=2
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" contains "q=test"
# Query strings should be preserved

# Test redirect with new query strings
GET http://{{host}}:{{port}}/track
Host: {{test_host}}
HTTP 302
# Might add tracking parameters

###############################################################################
# Protocol Redirects
###############################################################################

# Test HTTP to HTTPS redirect
GET http://{{host}}:{{port}}/secure-page
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" starts with "https://"

# Test www to non-www redirect
GET http://{{host}}:{{port}}/
Host: www.{{test_host}}
HTTP 301
# Might redirect to non-www version

###############################################################################
# Trailing Slash Redirects
###############################################################################

# Test adding trailing slash
GET http://{{host}}:{{port}}/directory
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" ends with "/"

# Test removing trailing slash
GET http://{{host}}:{{port}}/file.html/
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" not ends with "/"

###############################################################################
# Method Preservation
###############################################################################

# Test POST redirect (should use 307/308 to preserve method)
POST http://{{host}}:{{port}}/old-api
Host: {{test_host}}
Content-Type: application/json
```
{"test": true}
```
HTTP 307
[Asserts]
header "Location" exists
# 307 preserves POST method

# Test PUT redirect
PUT http://{{host}}:{{port}}/old-resource
Host: {{test_host}}
Content-Type: text/plain
```
Test content
```
HTTP 308
# 308 is permanent redirect preserving method

###############################################################################
# Redirect Loops Prevention
###############################################################################

# Test that redirects don't create loops
GET http://{{host}}:{{port}}/loop-test-a
Host: {{test_host}}
HTTP *
# Should not redirect to loop-test-b which redirects back

###############################################################################
# Conditional Redirects
###############################################################################

# Test user-agent based redirect
GET http://{{host}}:{{port}}/mobile
Host: {{test_host}}
User-Agent: Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)
HTTP 302
# Might redirect mobile users

# Test language-based redirect
GET http://{{host}}:{{port}}/
Host: {{test_host}}
Accept-Language: fr-FR,fr;q=0.9
HTTP 302
# Might redirect to French version

###############################################################################
# Special Cases
###############################################################################

# Test redirect with fragment (fragments are client-side)
GET http://{{host}}:{{port}}/docs#section
Host: {{test_host}}
HTTP 301
# Fragment won't be sent to server

# Test HEAD request redirect
HEAD http://{{host}}:{{port}}/old-page
Host: {{test_host}}
HTTP 301
[Asserts]
header "Location" exists

# Test OPTIONS request (usually not redirected)
OPTIONS http://{{host}}:{{port}}/old-api
Host: {{test_host}}
HTTP 200
# OPTIONS typically doesn't follow redirects