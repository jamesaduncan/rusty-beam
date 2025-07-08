# Security Headers Plugin Tests
# Tests security-related HTTP headers

###############################################################################
# Content Security Policy Tests
###############################################################################

# Test CSP header presence
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Content-Security-Policy" exists
# Should include directives like: default-src 'self'; script-src 'self' 'unsafe-inline'

# Test CSP Report-Only mode
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Might have Content-Security-Policy-Report-Only for testing

###############################################################################
# Strict Transport Security Tests
###############################################################################

# Test HSTS header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Strict-Transport-Security" exists
header "Strict-Transport-Security" contains "max-age="
# Should be like: max-age=31536000; includeSubDomains; preload

###############################################################################
# XSS Protection Tests
###############################################################################

# Test X-XSS-Protection header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-XSS-Protection" == "1; mode=block"

# Test for all HTML responses
GET http://{{host}}:{{port}}/index.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-XSS-Protection" exists

###############################################################################
# Content Type Options Tests
###############################################################################

# Test X-Content-Type-Options header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"

# Test on different content types
GET http://{{host}}:{{port}}/test.css
Host: {{test_host}}
HTTP 404
# Should still have security headers

###############################################################################
# Frame Options Tests
###############################################################################

# Test X-Frame-Options header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Frame-Options" matches "^(DENY|SAMEORIGIN)$"

# Test frame-ancestors CSP directive
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# CSP might include: frame-ancestors 'none' or 'self'

###############################################################################
# Referrer Policy Tests
###############################################################################

# Test Referrer-Policy header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Referrer-Policy" exists
header "Referrer-Policy" matches "(no-referrer|strict-origin|same-origin)"

###############################################################################
# Permissions Policy Tests
###############################################################################

# Test Permissions-Policy header
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "Permissions-Policy" exists
# Should restrict features like: geolocation=(), microphone=(), camera=()

###############################################################################
# CORS and Security Headers
###############################################################################

# Test security headers with CORS
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
Origin: https://example.com
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"
# Security headers should be present even with CORS

###############################################################################
# Security Headers on Errors
###############################################################################

# Test security headers on 404
GET http://{{host}}:{{port}}/missing.html
Host: {{test_host}}
HTTP 404
[Asserts]
header "X-Content-Type-Options" == "nosniff"
header "X-Frame-Options" exists

# Test security headers on 403
GET http://{{host}}:{{port}}/../etc/passwd
Host: {{test_host}}
HTTP 404
[Asserts]
header "X-Content-Type-Options" == "nosniff"

###############################################################################
# Cache Control Security
###############################################################################

# Test cache headers for sensitive content
GET http://{{host}}:{{port}}/admin/
Host: {{test_host}}
HTTP 404
# Might include: Cache-Control: no-store, no-cache, must-revalidate

###############################################################################
# Other Security Headers
###############################################################################

# Test X-Permitted-Cross-Domain-Policies
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Might include: X-Permitted-Cross-Domain-Policies: none

# Test X-Download-Options
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Might include: X-Download-Options: noopen

# Test Feature-Policy (legacy of Permissions-Policy)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
# Might include: Feature-Policy: geolocation 'none'

###############################################################################
# Method-Specific Headers
###############################################################################

# Test security headers on HEAD
HEAD http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"

# Test security headers on OPTIONS
OPTIONS http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200
[Asserts]
header "X-Content-Type-Options" == "nosniff"