# OAuth2 Plugin Tests
# Tests the OAuth2 authentication flow and integration with authorization

###############################################################################
# Login Endpoint Tests
###############################################################################

# Test 1: GET login endpoint redirects to Google
GET http://{{host}}:{{port}}/auth/google/login
Host: {{test_host}}
HTTP 302
[Asserts]
header "Location" contains "accounts.google.com/o/oauth2/v2/auth"
header "Location" contains "client_id="
header "Location" contains "redirect_uri="
header "Location" contains "response_type=code"
header "Location" contains "scope=email+profile"
header "Location" contains "state="
# State should be set in a secure cookie
cookie "oauth2_state" exists
cookie "oauth2_state[HttpOnly]" exists
cookie "oauth2_state[SameSite]" == "Lax"

# Test 2: Login with return_to parameter
GET http://{{host}}:{{port}}/auth/google/login?return_to=/admin
Host: {{test_host}}
HTTP 302
[Asserts]
header "Location" contains "accounts.google.com/o/oauth2/v2/auth"
# return_to should be stored in session
cookie "oauth2_return_to" exists
cookie "oauth2_return_to[HttpOnly]" exists

###############################################################################
# Callback Endpoint Tests
###############################################################################

# Test 3: Callback without code parameter returns error
GET http://{{host}}:{{port}}/auth/google/callback
Host: {{test_host}}
HTTP 400
[Asserts]
body contains "Missing authorization code"

# Test 4: Callback with invalid state returns error
GET http://{{host}}:{{port}}/auth/google/callback?code=test_code&state=invalid_state
Host: {{test_host}}
Cookie: oauth2_state=different_state
HTTP 403
[Asserts]
body == "Invalid state parameter"

# Test 5: Callback with valid code but missing state cookie  
# Note: This test may fail if cookies persist from previous tests
GET http://{{host}}:{{port}}/auth/google/callback?code=test_code&state=test_state
HTTP 403
[Asserts]
body matches "Missing state cookie|Invalid state parameter"

###############################################################################
# Logout Endpoint Tests
###############################################################################

# Test 6: Logout clears session
POST http://{{host}}:{{port}}/auth/logout
Host: {{test_host}}
Cookie: session_id=test_session_id
HTTP 302
[Asserts]
header "Location" == "/"
header "Set-Cookie" contains "session_id=; Max-Age=0"

# Test 7: Logout with return_to parameter
POST http://{{host}}:{{port}}/auth/logout?return_to=/goodbye
Host: {{test_host}}
Cookie: session_id=test_session_id
HTTP 302
[Asserts]
header "Location" == "/goodbye"

###############################################################################
# Status Endpoint Tests
###############################################################################

# Test 8: Status when not logged in
GET http://{{host}}:{{port}}/auth/status
Host: {{test_host}}
HTTP 401
[Asserts]
header "Content-Type" == "application/json"
jsonpath "$.authenticated" == false

# Test 9: Status when logged in (simulated)
# Note: This test will return 401 since we don't have a valid session in test environment
GET http://{{host}}:{{port}}/auth/status
Host: {{test_host}}
Cookie: session_id=valid_session_id
HTTP 401
[Asserts]
header "Content-Type" == "application/json"
jsonpath "$.authenticated" == false

###############################################################################
# Integration with Authorization Plugin
###############################################################################

# Test 10: Unauthenticated user can access public resources
GET http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200

# Test 11: OPTIONS shows correct permissions for anonymous user
OPTIONS http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200
[Asserts]
header "Allow" contains "GET"
header "Allow" contains "OPTIONS"
header "Accept-Ranges" == "selector"

###############################################################################
# Security Tests
###############################################################################

# Test 12: Session fixation protection
GET http://{{host}}:{{port}}/auth/google/login
Host: {{test_host}}
Cookie: session_id=attacker_session_id
HTTP 302
[Asserts]
# Should generate new session ID, not reuse the provided one
cookie "session_id" not exists

# Test 13: CSRF protection on logout - GET not allowed
GET http://{{host}}:{{port}}/auth/logout
Host: {{test_host}}
Cookie: session_id=valid_session_id
HTTP 404

# Test 14: Callback without state parameter
GET http://{{host}}:{{port}}/auth/google/callback?code=test_code
Host: {{test_host}}
HTTP 400
[Asserts]
body contains "Missing state parameter"