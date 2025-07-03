# Resource Selector Syntax

Resource selectors allow you to embed CSS selectors directly in URLs using the special `#(selector=...)` syntax. This provides an alternative to Range headers and enables more granular authorization control at the resource level.

## Overview

Resource selectors extend standard URLs with embedded CSS selector information:

**Standard URL**: `http://localhost:3000/page.html`  
**Resource Selector URL**: `http://localhost:3000/page.html#(selector=div.content)`

This syntax allows:
- Direct URL-based element targeting
- Granular authorization rules
- Bookmarkable selector-specific resources
- RESTful API design for HTML elements

## Basic Syntax

```
http://host:port/path/to/file.html#(selector=<css-selector>)
```

Where `<css-selector>` is any valid CSS selector, URL-encoded if necessary.

## Usage Examples

### Basic Element Selection

```bash
# Select by ID
curl http://localhost:3000/page.html#(selector=%23main-content)

# Select by class
curl http://localhost:3000/page.html#(selector=.content)

# Select by element type
curl http://localhost:3000/page.html#(selector=article)
```

### Complex Selectors

```bash
# Descendant selectors
curl "http://localhost:3000/page.html#(selector=article%20.content)"

# Child selectors  
curl "http://localhost:3000/page.html#(selector=nav%20%3E%20ul)"

# Attribute selectors
curl "http://localhost:3000/page.html#(selector=%5Bdata-role%3D%22button%22%5D)"

# Pseudo-selectors
curl "http://localhost:3000/page.html#(selector=li%3Afirst-child)"
```

### Multiple Operations

```bash
# GET: Retrieve specific elements
curl http://localhost:3000/admin.html#(selector=.admin-panel)

# PUT: Update specific elements
curl -X PUT \
  -H "Content-Type: text/html" \
  -d "<div class='updated'>New content</div>" \
  "http://localhost:3000/page.html#(selector=%23status)"

# POST: Append to specific elements
curl -X POST \
  -d "<li>New item</li>" \
  "http://localhost:3000/page.html#(selector=ul.items)"

# DELETE: Remove specific elements
curl -X DELETE \
  "http://localhost:3000/page.html#(selector=.temporary)"
```

## URL Encoding Requirements

Special characters in CSS selectors must be URL-encoded:

### Common Encodings

| Character | Encoded | Usage |
|-----------|---------|-------|
| `#` | `%23` | ID selectors |
| `.` | `.` | Class selectors (no encoding needed) |
| ` ` (space) | `%20` | Descendant combinators |
| `>` | `%3E` | Child combinators |
| `:` | `%3A` | Pseudo-selectors |
| `[` | `%5B` | Attribute selectors |
| `]` | `%5D` | Attribute selectors |
| `=` | `%3D` | Attribute values |
| `"` | `%22` | Quoted attribute values |

### Encoding Examples

```bash
# Original: #main > .content
# Encoded:  %23main%20%3E%20.content
curl "http://localhost:3000/page.html#(selector=%23main%20%3E%20.content)"

# Original: [data-role="button"]  
# Encoded:  %5Bdata-role%3D%22button%22%5D
curl "http://localhost:3000/page.html#(selector=%5Bdata-role%3D%22button%22%5D)"

# Original: li:nth-child(2n+1)
# Encoded:  li%3Anth-child%282n%2B1%29
curl "http://localhost:3000/page.html#(selector=li%3Anth-child%282n%2B1%29)"
```

## Authorization Integration

Resource selectors integrate seamlessly with Rusty-beam's authorization system, allowing fine-grained access control:

### Authorization Rules

```html
<!-- Allow admin access to admin panel selector -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">admin</td>
    <td itemprop="resource">/admin.html#(selector=.admin-panel)</td>
    <td itemprop="method">GET</td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Allow users to edit their own profile sections -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">:username</td>
    <td itemprop="resource">/users/:username/profile.html#(selector=.editable)</td>
    <td itemprop="method">PUT</td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Deny access to sensitive elements -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*#(selector=.sensitive)</td>
    <td itemprop="method">*</td>
    <td itemprop="permission">deny</td>
</tr>
```

### Authorization Examples

```bash
# Admin accessing admin panel (requires admin role)
curl -u admin:admin123 \
  "http://localhost:3000/admin.html#(selector=.admin-panel)"

# User accessing their profile section
curl -u johndoe:password \
  "http://localhost:3000/users/johndoe/profile.html#(selector=.personal-info)"

# Blocked access to sensitive content
curl -u guest:guest \
  "http://localhost:3000/page.html#(selector=.sensitive)"
# Returns: 403 Forbidden
```

## Comparison with Range Headers

Resource selectors and Range headers can be used independently or together:

### Resource Selector Only

```bash
# Simple selector in URL
curl "http://localhost:3000/page.html#(selector=.content)"
```

### Range Header Only

```bash
# Selector in header
curl -H "Range: selector=.content" \
  http://localhost:3000/page.html
```

### Combined Usage

```bash
# URL selector for authorization, Range for refinement
curl -H "Range: selector=p:first-child" \
  "http://localhost:3000/page.html#(selector=.content)"
```

### When to Use Each

| Use Case | Method | Reason |
|----------|--------|--------|
| Bookmarkable URLs | Resource selector | URLs are shareable and bookmarkable |
| Authorization rules | Resource selector | More granular permission control |
| Dynamic selection | Range header | Easier to programmatically modify |
| API consistency | Range header | Follows HTTP Range semantics |
| Complex workflows | Combined | Best of both approaches |

## Response Behavior

### Successful Selection

Resource selectors return the same response format as Range headers:

```http
HTTP/1.1 206 Partial Content
Content-Type: text/html
Content-Range: selector=.content
Content-Length: 234

<div class="content">
  <h2>Selected Content</h2>
  <p>This is the selected content.</p>
</div>
```

### Error Responses

| Scenario | Status | Description |
|----------|--------|-------------|
| Invalid selector syntax | 400 | Malformed CSS selector |
| No matching elements | 416 | Selector doesn't match anything |
| Access denied | 403 | Authorization rules prevent access |
| Non-HTML file | 400 | Selectors only work on HTML |

### Error Examples

```bash
# Invalid selector syntax
curl "http://localhost:3000/page.html#(selector=div..invalid)"
# HTTP/1.1 400 Bad Request

# No matching elements  
curl "http://localhost:3000/page.html#(selector=.nonexistent)"
# HTTP/1.1 416 Range Not Satisfiable

# Access denied
curl -u guest:guest \
  "http://localhost:3000/admin.html#(selector=.admin-only)"
# HTTP/1.1 403 Forbidden
```

## Security Model

### Authorization Granularity

Resource selectors enable authorization at the CSS selector level:

```html
<!-- Different permissions for different selectors -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editor</td>
    <td itemprop="resource">/article.html#(selector=.content)</td>
    <td itemprop="method">PUT</td>
    <td itemprop="permission">allow</td>
</tr>

<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editor</td>
    <td itemprop="resource">/article.html#(selector=.metadata)</td>
    <td itemprop="method">PUT</td>
    <td itemprop="permission">deny</td>
</tr>
```

### Wildcard Patterns

Authorization supports wildcard patterns in resource selectors:

```html
<!-- Allow access to any content selector -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">content-editor</td>
    <td itemprop="resource">/*#(selector=.content)</td>
    <td itemprop="method">PUT</td>
    <td itemprop="permission">allow</td>
</tr>

<!-- Deny access to admin selectors site-wide -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/*#(selector=.admin-*)</td>
    <td itemprop="method">*</td>
    <td itemprop="permission">deny</td>
</tr>
```

## Performance Considerations

### Caching

Resource selectors affect caching behavior:

- Each unique selector creates a distinct cache entry
- Selector parsing is cached per request
- Authorization checks are performed for each unique resource selector

### Efficiency Tips

```bash
# Efficient: Specific selectors
curl "http://localhost:3000/page.html#(selector=%23specific-id)"

# Less efficient: Broad selectors
curl "http://localhost:3000/page.html#(selector=*)"

# Good: Class-based selection
curl "http://localhost:3000/page.html#(selector=.content)"
```

## Best Practices

### 1. Use Meaningful Selectors

```bash
# Good: Semantic selector
/dashboard.html#(selector=.user-stats)

# Avoid: Generic selector
/dashboard.html#(selector=div)
```

### 2. Design Authorization-Friendly URLs

```bash
# Good: Clear permission boundaries
/admin/users.html#(selector=.user-list)    # View users
/admin/users.html#(selector=.user-actions) # Admin actions

# Better: Separate by function
/users/list.html#(selector=.user-row)
/admin/user-management.html#(selector=.admin-controls)
```

### 3. URL Encode Properly

Always URL encode special characters in production:

```python
# Python example
import urllib.parse

def build_resource_url(base_url, selector):
    encoded_selector = urllib.parse.quote(selector)
    return f"{base_url}#(selector={encoded_selector})"

url = build_resource_url(
    "http://localhost:3000/page.html",
    "#main > .content p:first-child"
)
# Result: http://localhost:3000/page.html#(selector=%23main%20%3E%20.content%20p%3Afirst-child)
```

### 4. Design for Bookmarkability

Resource selector URLs should be meaningful and bookmarkable:

```bash
# Good: Bookmarkable content URLs
http://blog.example.com/posts/123.html#(selector=.article-content)
http://docs.example.com/api.html#(selector=%23authentication-section)

# Good: RESTful element access
http://api.example.com/users/profile.html#(selector=.contact-info)
```

## Integration Examples

### JavaScript Client

```javascript
// Build resource selector URLs
function buildResourceURL(baseURL, selector) {
    const encoded = encodeURIComponent(selector);
    return `${baseURL}#(selector=${encoded})`;
}

// Fetch specific content
async function fetchContent(url, selector) {
    const resourceURL = buildResourceURL(url, selector);
    const response = await fetch(resourceURL);
    
    if (response.status === 206) {
        return await response.text();
    } else if (response.status === 416) {
        throw new Error('No content found for selector');
    } else if (response.status === 403) {
        throw new Error('Access denied');
    } else {
        throw new Error(`HTTP ${response.status}`);
    }
}

// Usage
try {
    const content = await fetchContent(
        'http://localhost:3000/page.html',
        '.main-content'
    );
    document.getElementById('target').innerHTML = content;
} catch (error) {
    console.error('Failed to fetch content:', error);
}
```

### cURL Scripts

```bash
#!/bin/bash
# fetch-element.sh - Fetch specific HTML elements

BASE_URL="http://localhost:3000"
FILE_PATH="$1"
SELECTOR="$2"
USERNAME="$3"
PASSWORD="$4"

if [ -z "$SELECTOR" ]; then
    echo "Usage: $0 <file-path> <css-selector> [username] [password]"
    exit 1
fi

# URL encode the selector
ENCODED_SELECTOR=$(printf '%s' "$SELECTOR" | curl -Gso /dev/null -w %{url_effective} --data-urlencode @- <<< "" | cut -c 3-)

# Build the resource URL
RESOURCE_URL="${BASE_URL}${FILE_PATH}#(selector=${ENCODED_SELECTOR})"

# Make the request
if [ -n "$USERNAME" ]; then
    curl -u "${USERNAME}:${PASSWORD}" "$RESOURCE_URL"
else
    curl "$RESOURCE_URL"
fi
```

## Advanced Patterns

### Conditional Access

```html
<!-- Time-based access example -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">*</td>
    <td itemprop="resource">/maintenance.html#(selector=.maintenance-notice)</td>
    <td itemprop="method">GET</td>
    <td itemprop="permission">allow</td>
</tr>
```

### Hierarchical Permissions

```html
<!-- Nested permission structure -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">viewer</td>
    <td itemprop="resource">/content/*#(selector=.public)</td>
    <td itemprop="method">GET</td>
    <td itemprop="permission">allow</td>
</tr>

<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">editor</td>
    <td itemprop="resource">/content/*#(selector=.editable)</td>
    <td itemprop="method">PUT</td>
    <td itemprop="permission">allow</td>
</tr>

<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">admin</td>
    <td itemprop="resource">/content/*#(selector=*)</td>
    <td itemprop="method">*</td>
    <td itemprop="permission">allow</td>
</tr>
```

## See Also

- [Range Selector Syntax](range-selectors.md) - Alternative selector method
- [Authorization System](../auth/authorization.md) - Permission control
- [CSS Selector Reference](css-selectors.md) - Supported selector syntax
- [API Examples](../examples/api-examples.md) - Real-world usage patterns