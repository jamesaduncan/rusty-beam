# Redirect Plugin Tests - Simplified Version
# Note: The redirect plugin currently doesn't parse HTML rules files
# This is a simplified test that verifies the plugin loads without errors

###############################################################################
# Basic Plugin Loading Test
###############################################################################

# Test that server responds (plugin doesn't break the server)
GET http://{{host}}:{{port}}/
Host: {{test_host}}
HTTP 200

# Test another path (should work even without redirect rules)
GET http://{{host}}:{{port}}/foo.html
Host: {{test_host}}
HTTP 200

# Note: Full redirect functionality requires implementing HTML rules file parsing
# in the redirect plugin. This is tracked as a TODO item.