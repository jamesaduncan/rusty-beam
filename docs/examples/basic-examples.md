# Basic Examples

This section provides practical examples of using Rusty-beam's CSS selector API for common web development tasks. These examples demonstrate both Range header and resource selector approaches.

## 📋 Table of Contents

1. [Getting Started](#getting-started)
2. [Content Retrieval](#content-retrieval)
3. [Content Updates](#content-updates)
4. [Dynamic Lists](#dynamic-lists)
5. [Form Interactions](#form-interactions)
6. [Authentication Examples](#authentication-examples)
7. [Error Handling](#error-handling)

## 🚀 Getting Started

### Start the Server

```bash
# Start Rusty-beam
rusty-beam

# Verify it's running
curl http://localhost:3000/
```

### Sample HTML File

Create a test file `localhost/test.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>Rusty-beam Test Page</title>
</head>
<body>
    <div id="main">
        <h1 class="title">Welcome to Rusty-beam</h1>
        <div class="content">
            <p>This is a test page for demonstrating CSS selectors.</p>
            <ul id="items">
                <li class="item">Item 1</li>
                <li class="item">Item 2</li>
                <li class="item">Item 3</li>
            </ul>
        </div>
        <div class="sidebar">
            <h2>Sidebar</h2>
            <p>This is the sidebar content.</p>
        </div>
    </div>
    <footer id="footer">
        <p>&copy; 2024 Rusty-beam Example</p>
    </footer>
</body>
</html>
```

## 📖 Content Retrieval

### Get Entire Elements

```bash
# Get the main content div
curl -H "Range: selector=#main" http://localhost:3000/test.html

# Get all paragraphs
curl -H "Range: selector=p" http://localhost:3000/test.html

# Get elements by class
curl -H "Range: selector=.content" http://localhost:3000/test.html
```

### Get Specific Text Content

```bash
# Get just the title
curl -H "Range: selector=h1.title" http://localhost:3000/test.html

# Get footer text
curl -H "Range: selector=#footer p" http://localhost:3000/test.html

# Get first paragraph
curl -H "Range: selector=.content p:first-child" http://localhost:3000/test.html
```

### Using Resource Selectors

```bash
# Alternative syntax using URL fragments
curl "http://localhost:3000/test.html#(selector=%23main)"

# Get sidebar content
curl "http://localhost:3000/test.html#(selector=.sidebar)"

# Get list items
curl "http://localhost:3000/test.html#(selector=%23items%20li)"
```

## ✏️ Content Updates

### Replace Element Content

```bash
# Update the title
curl -X PUT \
  -H "Range: selector=h1.title" \
  -H "Content-Type: text/html" \
  -d "<h1 class='title'>Updated Title</h1>" \
  http://localhost:3000/test.html

# Update a paragraph
curl -X PUT \
  -H "Range: selector=.content p" \
  -d "<p>This content has been updated via PUT request.</p>" \
  http://localhost:3000/test.html

# Update footer
curl -X PUT \
  -H "Range: selector=#footer" \
  -d "<footer id='footer'><p>&copy; 2024 Updated via API</p></footer>" \
  http://localhost:3000/test.html
```

### Update with Resource Selectors

```bash
# Update using URL-based selectors
curl -X PUT \
  -H "Content-Type: text/html" \
  -d "<h2>Updated Sidebar Title</h2>" \
  "http://localhost:3000/test.html#(selector=.sidebar%20h2)"
```

### Conditional Updates

```bash
# Update only if element exists (will return 416 if not found)
curl -X PUT \
  -H "Range: selector=.optional-element" \
  -d "<div class='optional-element'>New content</div>" \
  http://localhost:3000/test.html

# Check response code
if [ $? -eq 0 ]; then
    echo "Update successful"
else
    echo "Element not found or update failed"
fi
```

## 📝 Dynamic Lists

### Add Items to Lists

```bash
# Add new item to the end of list
curl -X POST \
  -H "Range: selector=#items" \
  -d "<li class='item'>New Item</li>" \
  http://localhost:3000/test.html

# Add multiple items
curl -X POST \
  -H "Range: selector=#items" \
  -d "<li class='item'>Item A</li><li class='item'>Item B</li>" \
  http://localhost:3000/test.html
```

### Remove List Items

```bash
# Remove specific item by content
curl -X DELETE \
  -H "Range: selector=#items li:contains('Item 2')" \
  http://localhost:3000/test.html

# Remove first item
curl -X DELETE \
  -H "Range: selector=#items li:first-child" \
  http://localhost:3000/test.html

# Remove all items with specific class
curl -X DELETE \
  -H "Range: selector=.item" \
  http://localhost:3000/test.html
```

### Update Specific List Items

```bash
# Update the second list item
curl -X PUT \
  -H "Range: selector=#items li:nth-child(2)" \
  -d "<li class='item updated'>Updated Item 2</li>" \
  http://localhost:3000/test.html
```

## 📋 Form Interactions

### Create a Form Example

Add this to your `test.html`:

```html
<form id="contact-form" class="form">
    <div class="field">
        <label for="name">Name:</label>
        <input type="text" id="name" name="name" value="">
    </div>
    <div class="field">
        <label for="email">Email:</label>
        <input type="email" id="email" name="email" value="">
    </div>
    <div class="field">
        <label for="message">Message:</label>
        <textarea id="message" name="message"></textarea>
    </div>
    <div class="status"></div>
    <button type="submit">Submit</button>
</form>
```

### Update Form Fields

```bash
# Pre-fill form fields
curl -X PUT \
  -H "Range: selector=#name" \
  -d '<input type="text" id="name" name="name" value="John Doe">' \
  http://localhost:3000/test.html

curl -X PUT \
  -H "Range: selector=#email" \
  -d '<input type="email" id="email" name="email" value="john@example.com">' \
  http://localhost:3000/test.html
```

### Update Form Status

```bash
# Show success message
curl -X PUT \
  -H "Range: selector=.status" \
  -d '<div class="status success">Form submitted successfully!</div>' \
  http://localhost:3000/test.html

# Show error message
curl -X PUT \
  -H "Range: selector=.status" \
  -d '<div class="status error">Please check your input.</div>' \
  http://localhost:3000/test.html

# Clear status
curl -X PUT \
  -H "Range: selector=.status" \
  -d '<div class="status"></div>' \
  http://localhost:3000/test.html
```

## 🔐 Authentication Examples

### Basic Authentication

```bash
# Access protected content
curl -u admin:admin123 \
  -H "Range: selector=.admin-panel" \
  http://localhost:3000/admin.html

# Try without credentials (should get 401)
curl -H "Range: selector=.admin-panel" \
  http://localhost:3000/admin.html
```

### User-Specific Content

```bash
# Access user's own profile section
curl -u johndoe:password \
  -H "Range: selector=.profile-info" \
  http://localhost:3000/users/johndoe/profile.html

# Update user's own data
curl -u johndoe:password \
  -X PUT \
  -H "Range: selector=.contact-info" \
  -d '<div class="contact-info">Updated contact information</div>' \
  http://localhost:3000/users/johndoe/profile.html
```

## ⚠️ Error Handling

### Handle Missing Elements

```bash
# Attempt to select non-existent element
response=$(curl -s -w "%{http_code}" \
  -H "Range: selector=.nonexistent" \
  http://localhost:3000/test.html)

echo "Response: $response"
# Should show: 416 Range Not Satisfiable
```

### Handle Invalid Selectors

```bash
# Invalid CSS selector
response=$(curl -s -w "%{http_code}" \
  -H "Range: selector=div..invalid" \
  http://localhost:3000/test.html)

echo "Response: $response"
# Should show: 400 Bad Request
```

### Robust Error Handling Script

```bash
#!/bin/bash
# robust-update.sh - Update with error handling

URL="http://localhost:3000/test.html"
SELECTOR="$1"
CONTENT="$2"
USERNAME="$3"
PASSWORD="$4"

if [ -z "$CONTENT" ]; then
    echo "Usage: $0 <selector> <content> [username] [password]"
    exit 1
fi

# Build curl command
CURL_CMD="curl -s -w '%{http_code}' -X PUT"
CURL_CMD="$CURL_CMD -H 'Range: selector=$SELECTOR'"
CURL_CMD="$CURL_CMD -H 'Content-Type: text/html'"

if [ -n "$USERNAME" ]; then
    CURL_CMD="$CURL_CMD -u $USERNAME:$PASSWORD"
fi

CURL_CMD="$CURL_CMD -d '$CONTENT' '$URL'"

# Execute and capture response
response=$(eval $CURL_CMD)
http_code="${response: -3}"
body="${response%???}"

case "$http_code" in
    "206")
        echo "✓ Content updated successfully"
        ;;
    "400")
        echo "✗ Bad request - check your CSS selector"
        echo "Response: $body"
        ;;
    "401")
        echo "✗ Authentication required"
        ;;
    "403")
        echo "✗ Access denied"
        ;;
    "416")
        echo "✗ No elements found matching selector: $SELECTOR"
        ;;
    *)
        echo "✗ Unexpected response: $http_code"
        echo "Response: $body"
        ;;
esac
```

## 🔄 Real-time Updates

### Polling for Changes

```bash
#!/bin/bash
# poll-content.sh - Poll for content changes

URL="http://localhost:3000/test.html"
SELECTOR=".status"
INTERVAL=5

echo "Polling $URL for changes in $SELECTOR every ${INTERVAL}s..."

last_content=""
while true; do
    current_content=$(curl -s -H "Range: selector=$SELECTOR" "$URL")
    
    if [ "$current_content" != "$last_content" ]; then
        echo "$(date): Content changed!"
        echo "$current_content"
        echo "---"
        last_content="$current_content"
    fi
    
    sleep $INTERVAL
done
```

### Watch for Element Presence

```bash
#!/bin/bash
# wait-for-element.sh - Wait for element to appear

URL="http://localhost:3000/test.html"
SELECTOR="$1"
TIMEOUT=30

if [ -z "$SELECTOR" ]; then
    echo "Usage: $0 <css-selector>"
    exit 1
fi

echo "Waiting for element matching '$SELECTOR' to appear..."

start_time=$(date +%s)
while true; do
    current_time=$(date +%s)
    elapsed=$((current_time - start_time))
    
    if [ $elapsed -gt $TIMEOUT ]; then
        echo "Timeout: Element not found after ${TIMEOUT}s"
        exit 1
    fi
    
    response=$(curl -s -w "%{http_code}" \
        -H "Range: selector=$SELECTOR" \
        "$URL")
    
    http_code="${response: -3}"
    
    if [ "$http_code" = "206" ]; then
        echo "✓ Element found!"
        body="${response%???}"
        echo "$body"
        exit 0
    fi
    
    sleep 1
done
```

## 📊 Batch Operations

### Update Multiple Elements

```bash
#!/bin/bash
# batch-update.sh - Update multiple elements

URL="http://localhost:3000/test.html"

# Array of selector:content pairs
updates=(
    "h1.title:<h1 class='title'>Batch Updated Title</h1>"
    ".content p:<p>Batch updated paragraph content.</p>"
    "#footer p:<p>&copy; 2024 Batch Updated Footer</p>"
)

echo "Performing batch updates..."

for update in "${updates[@]}"; do
    IFS=':' read -r selector content <<< "$update"
    
    echo "Updating $selector..."
    response=$(curl -s -w "%{http_code}" \
        -X PUT \
        -H "Range: selector=$selector" \
        -H "Content-Type: text/html" \
        -d "$content" \
        "$URL")
    
    http_code="${response: -3}"
    
    if [ "$http_code" = "206" ]; then
        echo "✓ Updated $selector"
    else
        echo "✗ Failed to update $selector (HTTP $http_code)"
    fi
done

echo "Batch update complete!"
```

### Backup and Restore

```bash
#!/bin/bash
# backup-restore.sh - Backup and restore specific elements

URL="http://localhost:3000/test.html"
BACKUP_DIR="./backups/$(date +%Y%m%d_%H%M%S)"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Selectors to backup
selectors=("h1.title" ".content" "#footer")

echo "Creating backup in $BACKUP_DIR..."

# Backup elements
for selector in "${selectors[@]}"; do
    filename=$(echo "$selector" | tr '.' '_' | tr '#' '_' | tr ' ' '_')
    
    echo "Backing up $selector..."
    curl -s -H "Range: selector=$selector" "$URL" > "$BACKUP_DIR/$filename.html"
    
    if [ $? -eq 0 ]; then
        echo "✓ Backed up $selector to $filename.html"
    else
        echo "✗ Failed to backup $selector"
    fi
done

echo "Backup complete!"

# Restore function
restore_from_backup() {
    local backup_dir="$1"
    
    if [ ! -d "$backup_dir" ]; then
        echo "Backup directory not found: $backup_dir"
        return 1
    fi
    
    echo "Restoring from $backup_dir..."
    
    for selector in "${selectors[@]}"; do
        filename=$(echo "$selector" | tr '.' '_' | tr '#' '_' | tr ' ' '_')
        backup_file="$backup_dir/$filename.html"
        
        if [ -f "$backup_file" ]; then
            echo "Restoring $selector..."
            content=$(cat "$backup_file")
            
            curl -s -X PUT \
                -H "Range: selector=$selector" \
                -H "Content-Type: text/html" \
                -d "$content" \
                "$URL" > /dev/null
            
            if [ $? -eq 0 ]; then
                echo "✓ Restored $selector"
            else
                echo "✗ Failed to restore $selector"
            fi
        else
            echo "✗ Backup file not found: $backup_file"
        fi
    done
    
    echo "Restore complete!"
}

# Uncomment to restore from a specific backup
# restore_from_backup "./backups/20241203_143022"
```

## 🧪 Testing and Validation

### Test Suite

```bash
#!/bin/bash
# test-suite.sh - Comprehensive test suite

URL="http://localhost:3000/test.html"
TESTS_PASSED=0
TESTS_FAILED=0

# Test function
run_test() {
    local test_name="$1"
    local expected_status="$2"
    local curl_command="$3"
    
    echo -n "Testing $test_name... "
    
    response=$(eval "$curl_command")
    actual_status="${response: -3}"
    
    if [ "$actual_status" = "$expected_status" ]; then
        echo "✓ PASS"
        ((TESTS_PASSED++))
    else
        echo "✗ FAIL (expected $expected_status, got $actual_status)"
        ((TESTS_FAILED++))
    fi
}

echo "Running Rusty-beam CSS Selector Test Suite"
echo "=========================================="

# Basic GET tests
run_test "Get existing element" "206" \
    "curl -s -w '%{http_code}' -H 'Range: selector=#main' '$URL'"

run_test "Get non-existent element" "416" \
    "curl -s -w '%{http_code}' -H 'Range: selector=.nonexistent' '$URL'"

run_test "Invalid CSS selector" "400" \
    "curl -s -w '%{http_code}' -H 'Range: selector=div..invalid' '$URL'"

# PUT tests
run_test "Update existing element" "206" \
    "curl -s -w '%{http_code}' -X PUT -H 'Range: selector=h1.title' -d '<h1 class=\"title\">Test Update</h1>' '$URL'"

run_test "Update non-existent element" "416" \
    "curl -s -w '%{http_code}' -X PUT -H 'Range: selector=.nonexistent' -d '<div>test</div>' '$URL'"

# POST tests
run_test "Append to existing element" "206" \
    "curl -s -w '%{http_code}' -X POST -H 'Range: selector=#items' -d '<li class=\"item\">Test Item</li>' '$URL'"

# DELETE tests
run_test "Delete existing element" "206" \
    "curl -s -w '%{http_code}' -X DELETE -H 'Range: selector=#items li:last-child' '$URL'"

# Resource selector tests
run_test "Resource selector GET" "206" \
    "curl -s -w '%{http_code}' '$URL#(selector=%23main)'"

echo "=========================================="
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $TESTS_FAILED"

if [ $TESTS_FAILED -eq 0 ]; then
    echo "All tests passed! ✓"
    exit 0
else
    echo "Some tests failed! ✗"
    exit 1
fi
```

These examples provide a solid foundation for working with Rusty-beam's CSS selector API. You can adapt and extend these patterns for your specific use cases.

## 📚 Next Steps

- Explore [Advanced Examples](advanced-examples.md) for more complex scenarios
- Learn about [Authentication](../auth/authentication.md) for securing your content
- Check out [Plugin Development](../plugins/writing-plugins.md) for extending functionality