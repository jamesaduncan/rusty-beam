# Range Selector Syntax

Rusty-beam's core innovation is the creative abuse of HTTP Range headers to specify CSS selectors for HTML element manipulation. This allows for powerful, RESTful HTML manipulation using familiar CSS selector syntax.

## Overview

Instead of using Range headers for byte ranges (their original purpose), Rusty-beam interprets them as CSS selectors to target specific HTML elements within documents.

**Standard HTTP Range**: `Range: bytes=0-1023`  
**Rusty-beam Range**: `Range: selector=div.content`

## Basic Syntax

```http
Range: selector=<css-selector>
```

Where `<css-selector>` is any valid CSS selector.

## Supported Operations

### GET with Range Selector

Extract specific HTML elements from a document.

```bash
# Get all paragraphs
curl -H "Range: selector=p" http://localhost:3000/page.html

# Get element by ID
curl -H "Range: selector=#main-content" http://localhost:3000/page.html

# Get elements by class
curl -H "Range: selector=.highlight" http://localhost:3000/page.html

# Complex selectors
curl -H "Range: selector=article > .content p:first-child" http://localhost:3000/page.html
```

### PUT with Range Selector

Replace the content of matching elements.

```bash
# Replace content of an element
curl -X PUT \
  -H "Range: selector=#status" \
  -H "Content-Type: text/html" \
  -d "<span class='success'>Updated!</span>" \
  http://localhost:3000/page.html

# Replace multiple elements
curl -X PUT \
  -H "Range: selector=.error-message" \
  -d "<div class='success'>All errors fixed!</div>" \
  http://localhost:3000/page.html
```

### POST with Range Selector

Append content to matching elements.

```bash
# Append to an element
curl -X POST \
  -H "Range: selector=#log" \
  -d "<p>New log entry at $(date)</p>" \
  http://localhost:3000/admin.html

# Add items to a list
curl -X POST \
  -H "Range: selector=ul.items" \
  -d "<li>New item</li>" \
  http://localhost:3000/page.html
```

### DELETE with Range Selector

Remove matching elements.

```bash
# Delete specific elements
curl -X DELETE \
  -H "Range: selector=.temporary" \
  http://localhost:3000/page.html

# Delete by ID
curl -X DELETE \
  -H "Range: selector=#old-banner" \
  http://localhost:3000/page.html
```

## CSS Selector Support

Rusty-beam supports the full CSS selector specification through the `dom_query` library:

### Basic Selectors

```bash
# Element selector
Range: selector=div

# Class selector  
Range: selector=.className

# ID selector
Range: selector=#elementId

# Attribute selector
Range: selector=[data-role="button"]

# Universal selector
Range: selector=*
```

### Combinators

```bash
# Descendant combinator
Range: selector=article p

# Child combinator
Range: selector=nav > ul

# Adjacent sibling
Range: selector=h1 + p

# General sibling
Range: selector=h1 ~ p
```

### Pseudo-classes

```bash
# Structural pseudo-classes
Range: selector=li:first-child
Range: selector=tr:nth-child(2n)
Range: selector=p:last-of-type

# State pseudo-classes (based on attributes)
Range: selector=input[disabled]
Range: selector=a[href^="https"]
```

### Complex Selectors

```bash
# Multiple selectors (comma-separated)
Range: selector=h1, h2, h3

# Complex combinations
Range: selector=.sidebar article[data-category="news"] > .content p:not(.meta)

# Attribute contains
Range: selector=[class*="btn"]

# Attribute ends with
Range: selector=[src$=".jpg"]
```

## URL Encoding

When using complex selectors, URL encoding may be necessary:

```bash
# Spaces need encoding
Range: selector=div%20.content

# Special characters
Range: selector=%23main%20%3E%20.content  # #main > .content

# Using curl's --data-urlencode for complex selectors
curl -G --data-urlencode "selector=article[data-id='123'] > .content" \
  -H "Range: selector=$(echo 'article[data-id="123"] > .content' | sed 's/ /%20/g')" \
  http://localhost:3000/page.html
```

## Response Behavior

### Successful Selection

When elements are found, Rusty-beam returns:

- **Status**: `206 Partial Content` (maintaining Range header semantics)
- **Content-Type**: `text/html`
- **Content-Range**: `selector=<original-selector>`
- **Body**: The selected HTML elements

```http
HTTP/1.1 206 Partial Content
Content-Type: text/html
Content-Range: selector=div.content
Content-Length: 156

<div class="content">
  <h2>Article Title</h2>
  <p>Article content here...</p>
</div>
```

### No Matches

When no elements match the selector:

- **Status**: `416 Range Not Satisfiable`
- **Body**: Error message

```http
HTTP/1.1 416 Range Not Satisfiable
Content-Type: text/plain

No elements found matching selector: div.nonexistent
```

### Invalid Selector

When the CSS selector is invalid:

- **Status**: `400 Bad Request`
- **Body**: Parse error details

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain

Invalid CSS selector: div..invalid
```

## Error Handling

### Common Errors

| Error | Status | Cause |
|-------|--------|-------|
| Invalid selector syntax | 400 | Malformed CSS selector |
| No matching elements | 416 | Selector doesn't match any elements |
| Non-HTML file | 400 | Trying to use selectors on non-HTML content |
| Permission denied | 403 | Authorization rules prevent access |

### Error Examples

```bash
# Invalid selector
curl -H "Range: selector=div..invalid" http://localhost:3000/page.html
# Returns: 400 Bad Request

# No matches
curl -H "Range: selector=.nonexistent" http://localhost:3000/page.html  
# Returns: 416 Range Not Satisfiable

# Non-HTML file
curl -H "Range: selector=p" http://localhost:3000/image.jpg
# Returns: 400 Bad Request
```

## Security Considerations

### Authorization Integration

Range selectors integrate with Rusty-beam's authorization system:

```html
<!-- Authorization rule example -->
<tr itemscope itemtype="http://rustybeam.net/Authorization">
    <td itemprop="username">admin</td>
    <td itemprop="resource">/admin.html#(selector=.admin-panel)</td>
    <td itemprop="method">GET</td>
    <td itemprop="permission">allow</td>
</tr>
```

### Selector Restrictions

Some selectors may be restricted based on:

- User roles and permissions
- Resource-specific rules
- Security policies

```bash
# May require admin privileges
curl -u admin:password \
  -H "Range: selector=.admin-only" \
  http://localhost:3000/admin.html
```

## Performance Notes

### Selector Efficiency

More specific selectors are generally faster:

```bash
# Efficient - ID selector
Range: selector=#specific-element

# Less efficient - Universal with filters
Range: selector=*[data-role="content"]

# Good balance - Class with element
Range: selector=div.content
```

### Caching Behavior

- Parsed CSS selectors are cached per request
- DOM parsing is performed once per file modification
- Response caching respects standard HTTP cache headers

## Best Practices

### 1. Use Specific Selectors

```bash
# Good: Specific and fast
Range: selector=#main-content

# Avoid: Too broad
Range: selector=*
```

### 2. Combine with HTTP Methods Semantically

```bash
# GET: Retrieve content
GET + Range: selector=.article-content

# PUT: Replace content  
PUT + Range: selector=#status + new content

# POST: Append content
POST + Range: selector=.comments + new comment

# DELETE: Remove content
DELETE + Range: selector=.temporary
```

### 3. Handle Errors Gracefully

Always check for Range Not Satisfiable responses:

```bash
response=$(curl -s -w "%{http_code}" -H "Range: selector=.content" http://localhost:3000/page.html)
if [[ "${response: -3}" == "416" ]]; then
    echo "No content found"
fi
```

### 4. URL Encode Complex Selectors

For programmatic access, always URL encode selectors:

```python
import urllib.parse
selector = "article[data-id='123'] > .content"
encoded = urllib.parse.quote(selector)
headers = {"Range": f"selector={encoded}"}
```

## Examples

See [Range Selector Examples](../examples/range-selector-examples.md) for comprehensive usage examples and real-world scenarios.