# Authorization System

Rusty-beam's authorization system provides fine-grained access control based on users, roles, resources, HTTP methods, and CSS selectors. The system integrates seamlessly with the authentication plugins and supports complex permission scenarios.

## Overview

Authorization in Rusty-beam operates after successful authentication and controls access to:

- **Files and directories**
- **Specific HTML elements** (via CSS selectors)
- **HTTP methods** (GET, PUT, POST, DELETE)
- **Resource patterns** (wildcard matching)

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Authenticated   │───►│ Authorization   │───►│ Resource Access │
│ User + Roles    │    │ Rules Engine    │    │ Granted/Denied  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Authorization Rules Format

### HTML Microdata Schema

Authorization rules are defined using HTML microdata in the same file as users:

```html
<table id="authorization">
    <thead>
        <tr>
            <td>Username</td>
            <td>Resource</td>
            <td>Method</td>
            <td>Permission</td>
        </tr>
    </thead>
    <tbody>
        <tr itemscope itemtype="http://rustybeam.net/Authorization">
            <td itemprop="username">admin</td>
            <td itemprop="resource">/admin/*</td>
            <td>
                <ul>
                    <li itemprop="method">GET</li>
                    <li itemprop="method">PUT</li>
                    <li itemprop="method">POST</li>
                    <li itemprop="method">DELETE</li>
                </ul>
            </td>
            <td itemprop="permission">allow</td>
        </tr>
    </tbody>
</table>
```

### Rule Schema

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `username` | string | Yes | User or role name, or `*` for all, or `:username` for dynamic |
| `resource` | string | Yes | Resource path pattern with optional selector |
| `method` | string[] | Yes | HTTP methods (GET, PUT, POST, DELETE, or `*`) |
| `permission` | string | Yes | `allow` or `deny` |

## Resource Patterns

### Basic Path Patterns

```html
<!-- Exact path match -->
<td itemprop="resource">/admin/users.html</td>

<!-- Directory wildcard -->
<td itemprop="resource">/admin/*</td>

<!-- Recursive wildcard -->
<td itemprop="resource">/content/**</td>

<!-- File extension pattern -->
<td itemprop="resource">*.html</td>

<!-- Root level files -->
<td itemprop="resource">/*</td>
```

### CSS Selector Resources

Resources can include CSS selectors for element-level permissions:

```html
<!-- Allow access to specific HTML elements -->
<td itemprop="resource">/page.html#(selector=.content)</td>

<!-- Wildcard selector permissions -->
<td itemprop="resource">/*#(selector=.public)</td>

<!-- Admin-only elements -->
<td itemprop="resource">/admin.html#(selector=.admin-panel)</td>

<!-- User-specific content -->
<td itemprop="resource">/users/:username/profile.html#(selector=.personal-info)</td>
```

### Dynamic Variables

Resources support dynamic variables that are replaced at runtime:

```html
<!-- :username is replaced with the authenticated user's username -->
<td itemprop="resource">/users/:username/*</td>

<!-- Multiple variables possible -->
<td itemprop="resource">/tenants/:tenant/users/:username/data/*</td>
```

## User and Role Matching

### Direct User Matching

```html
<!-- Specific user -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">johndoe</td>
    <td itemprop="resource">/users/johndoe/*</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### Role-based Matching

```html
<!-- Role-based access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">administrators</td>
    <td itemprop="resource">/admin/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### Wildcard Matching

```html
<!-- All users -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### Dynamic User Variables

```html
<!-- Current user variable -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">:username</td>
    <td itemprop="resource">/users/:username/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

## HTTP Method Permissions

### Single Method

```html
<td>
    <ul>
        <li itemprop="method">GET</li>
    </ul>
</td>
```

### Multiple Methods

```html
<td>
    <ul>
        <li itemprop="method">GET</li>
        <li itemprop="method">PUT</li>
        <li itemprop="method">POST</li>
    </ul>
</td>
```

### All Methods

```html
<td>
    <ul>
        <li itemprop="method">*</li>
    </ul>
</td>
```

## Rule Evaluation Engine

### Rule Precedence

Rules are evaluated in order of specificity (most specific first):

1. **User specificity**: Exact username > Role name > Wildcard (`*`)
2. **Path specificity**: Exact path > Path segments > Wildcards
3. **Method specificity**: Exact method > Wildcard method
4. **Selector specificity**: Exact selector > No selector

### Evaluation Algorithm

```rust
// Pseudo-code for rule evaluation
fn authorize(user: &User, resource: &str, method: &str) -> bool {
    let matching_rules = find_matching_rules(user, resource, method);
    
    // Sort by specificity (most specific first)
    matching_rules.sort_by_key(|rule| rule.specificity());
    
    // Apply first matching rule
    if let Some(rule) = matching_rules.first() {
        rule.permission == Permission::Allow
    } else {
        // Default deny
        false
    }
}
```

### Specificity Calculation

```rust
fn calculate_specificity(rule: &AuthorizationRule) -> i32 {
    let mut specificity = 0;
    
    // User specificity
    if rule.username != "*" { specificity += 1000; }
    
    // Path specificity
    specificity += rule.resource.split('/').count() * 10;
    if !rule.resource.contains('*') { specificity += 100; }
    
    // Selector specificity
    if rule.resource.contains("#(selector=") { specificity += 50; }
    
    // Method specificity
    if rule.methods.len() == 1 && !rule.methods.contains(&"*") {
        specificity += 5;
    }
    
    specificity
}
```

## Common Authorization Patterns

### 1. Hierarchical Permissions

```html
<!-- Base rule: Deny all admin access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/admin/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Override: Allow admins -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">administrators</td>
    <td itemprop="resource">/admin/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### 2. Read-Only Access

```html
<!-- Public read access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Deny write operations for non-editors -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td>
        <ul>
            <li itemprop="method">PUT</li>
            <li itemprop="method">POST</li>
            <li itemprop="method">DELETE</li>
        </ul>
    </td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Allow editors to write -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editors</td>
    <td itemprop="resource">/content/*</td>
    <td>
        <ul>
            <li itemprop="method">PUT</li>
            <li itemprop="method">POST</li>
        </ul>
    </td>
    <td itemprop="permission">allow</td>
</tr>
```

### 3. User-Specific Resources

```html
<!-- Users can access their own directories -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">:username</td>
    <td itemprop="resource">/users/:username/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- But deny access to other users' directories -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/users/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">deny</td>
</tr>
```

### 4. Element-Level Permissions

```html
<!-- Public content is viewable by all -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*#(selector=.public)</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Private content requires authentication -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*#(selector=.private)</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">deny</td>
</tr>

<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">members</td>
    <td itemprop="resource">/*#(selector=.private)</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Admin panels only for administrators -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">administrators</td>
    <td itemprop="resource">/*#(selector=.admin-panel)</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

## Advanced Authorization Scenarios

### 1. Content Management System

```html
<!-- Viewers can read public content -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">viewers</td>
    <td itemprop="resource">/content/*#(selector=.article-content)</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Editors can modify content but not metadata -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editors</td>
    <td itemprop="resource">/content/*#(selector=.article-content)</td>
    <td><li itemprop="method">PUT</li></td>
    <td itemprop="permission">allow</td>
</tr>

<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editors</td>
    <td itemprop="resource">/content/*#(selector=.article-metadata)</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Publishers can modify everything -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">publishers</td>
    <td itemprop="resource">/content/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### 2. Multi-tenant Application

```html
<!-- Tenant isolation -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/tenants/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Users can access their tenant's data -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">tenant-a-users</td>
    <td itemprop="resource">/tenants/tenant-a/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Tenant admins get full access -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">tenant-a-admins</td>
    <td itemprop="resource">/tenants/tenant-a/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### 3. API Endpoint Protection

```html
<!-- Public API endpoints -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/api/public/*</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Protected API endpoints -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">api-users</td>
    <td itemprop="resource">/api/v1/*</td>
    <td>
        <ul>
            <li itemprop="method">GET</li>
            <li itemprop="method">POST</li>
        </ul>
    </td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Admin API endpoints -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">api-admins</td>
    <td itemprop="resource">/api/admin/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

## Error Responses

### 403 Forbidden

When authorization fails:

```http
HTTP/1.1 403 Forbidden
Content-Type: text/plain
Content-Length: 13

Access denied
```

### 401 Unauthorized

When authentication is required but not provided:

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Basic realm="Rusty Beam"
Content-Type: text/plain
Content-Length: 23

Authentication required
```

## Testing Authorization Rules

### Manual Testing

```bash
# Test as admin user
curl -u admin:admin123 -X GET http://localhost:3000/admin/users.html
# Expected: 200 OK

# Test as regular user
curl -u johndoe:password -X GET http://localhost:3000/admin/users.html  
# Expected: 403 Forbidden

# Test selector-specific access
curl -u editor:password -H "Range: selector=.content" http://localhost:3000/article.html
# Expected: 206 Partial Content

# Test method restrictions
curl -u viewer:password -X PUT http://localhost:3000/content/article.html
# Expected: 403 Forbidden
```

### Automated Testing

Create test scripts to verify authorization rules:

```bash
#!/bin/bash
# test-authorization.sh

BASE_URL="http://localhost:3000"

# Test admin access
echo "Testing admin access..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u admin:admin123 "$BASE_URL/admin/")
if [ "$status" = "200" ]; then
    echo "✓ Admin access allowed"
else
    echo "✗ Admin access denied (got $status)"
fi

# Test user access to admin area
echo "Testing user access to admin area..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u user:password "$BASE_URL/admin/")
if [ "$status" = "403" ]; then
    echo "✓ User correctly denied admin access"
else
    echo "✗ User incorrectly allowed admin access (got $status)"
fi

# Test selector-based access
echo "Testing selector access..."
status=$(curl -s -o /dev/null -w "%{http_code}" -u editor:password -H "Range: selector=.content" "$BASE_URL/article.html")
if [ "$status" = "206" ]; then
    echo "✓ Editor can access content selector"
else
    echo "✗ Editor denied content selector access (got $status)"
fi
```

## Performance Considerations

### Rule Optimization

1. **Order Rules by Specificity**: Most specific rules first
2. **Minimize Wildcard Rules**: They require more processing
3. **Use Caching**: Rules are cached per host configuration

### Large Rule Sets

For applications with many authorization rules:

- Consider separating rules into multiple files
- Use role-based permissions instead of user-specific rules
- Implement rule compilation for better performance

## Debugging Authorization

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Common Issues

1. **Rules Not Matching**:
   - Check path patterns and wildcards
   - Verify user roles are correct
   - Ensure resource paths match exactly

2. **Conflicting Rules**:
   - Review rule specificity order
   - Check for overlapping patterns
   - Use explicit deny rules when needed

3. **Selector Issues**:
   - Verify CSS selector syntax
   - Check for URL encoding problems
   - Test selectors independently

### Debug Tools

```bash
# Test specific authorization scenarios
curl -v -u username:password \
  -H "Range: selector=.test" \
  http://localhost:3000/test.html

# Check rule parsing
grep -A 5 -B 5 "Authorization" /path/to/auth/file.html
```

## Security Best Practices

### 1. Principle of Least Privilege

```html
<!-- Start with deny-all, then grant specific permissions -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*</td>
    <td><li itemprop="method">*</li></td>
    <td itemprop="permission">deny</td>
</tr>

<!-- Grant only necessary permissions -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">readers</td>
    <td itemprop="resource">/public/*</td>
    <td><li itemprop="method">GET</li></td>
    <td itemprop="permission">allow</td>
</tr>
```

### 2. Defense in Depth

- **File System Permissions**: Restrict access at OS level
- **Network Segmentation**: Isolate sensitive resources
- **Input Validation**: Sanitize all user inputs
- **Audit Logging**: Track all authorization decisions

### 3. Regular Review

- Periodically audit authorization rules
- Remove unused or overprivileged rules
- Test with different user scenarios
- Monitor for unusual access patterns

## See Also

- [Authentication System](authentication.md) - User identity verification
- [User Management](user-management.md) - Managing users and roles
- [Resource Selectors](../api/resource-selectors.md) - CSS selector resources
- [Security Guide](../guides/security.md) - Production security practices