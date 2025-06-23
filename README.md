# Experimental HTTP Server with CSS Selector Support

A lightweight file server written in Rust that supports full CRUD operations via HTTP methods **with CSS selector-based HTML manipulation**.

Built with `hyper` for high-performance async HTTP handling, `tokio` for async runtime, and `dom_query` for HTML document manipulation.

## ✨ Key Features

### Standard File Operations
- **GET**: Serve files, index.html auto-serving, and directory listings
- **PUT**: Upload or completely overwrite files
- **POST**: Append content to existing directories  
- **DELETE**: Remove files or directories
- **OPTIONS**: CORS support with method discovery

### 🎯 Advanced HTML Manipulation (CSS Selectors)
- **GET + Selector**: Extract specific HTML elements using CSS selectors
- **PUT + Selector**: Replace content of specific HTML elements
- **POST + Selector**: Append content to specific HTML elements
- **DELETE + Selector**: Remove specific HTML elements from documents

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ 
- Cargo

## 🛠️ Usage Examples

### Basic File Operations

**Serve index.html automatically:**
```bash
curl http://127.0.0.1:3000/
# Serves ./files/index.html if it exists
```

**Upload an HTML file:**
```bash
curl -X PUT -d '<html><body><h1 id="title">Hello</h1><p class="content">World</p></body></html>' \
  http://127.0.0.1:3000/test.html
```

**Download the full file:**
```bash
curl http://127.0.0.1:3000/test.html
```

### 🎯 CSS Selector Operations

**Extract specific elements (GET + Selector):**
```bash
# Get only the h1 element
curl -H "Range: selector=#title" http://127.0.0.1:3000/test.html
# Returns: <h1 id="title">Hello</h1>

# Get elements by class
curl -H "Range: selector=.content" http://127.0.0.1:3000/test.html
# Returns: <p class="content">World</p>
```

**Replace element content (PUT + Selector):**
```bash
# Replace the h1 content
curl -X PUT -H "Range: selector=#title" \
  -d '<h1 id="title">New Title</h1>' \
  http://127.0.0.1:3000/test.html
```

**Append to elements (POST + Selector):**
```bash
# Append to the body
curl -X POST -H "Range: selector=body" \
  -d '<footer>Footer content</footer>' \
  http://127.0.0.1:3000/test.html
```

**Remove elements (DELETE + Selector):**
```bash
# Remove the paragraph
curl -X DELETE -H "Range: selector=.content" \
  http://127.0.0.1:3000/test.html
```

## 📡 API Reference

### Standard Operations

| Method | Endpoint | Description | Response |
|--------|----------|-------------|----------|
| OPTIONS | `/*` | Get allowed methods | 200 + Allow header |
| GET | `/{path}` | Serve file or index.html | File content |
| PUT | `/{path}` | Upload/overwrite file | 201 Created |
| POST | `/{path}` | Append to file | 200 OK |
| DELETE | `/{path}` | Delete file/directory | 200 OK |

### CSS Selector Operations (HTML files only)

| Method | Header | Description | Response |
|--------|--------|-------------|----------|
| GET | `Range: selector={css}` | Extract matching elements | HTML fragment |
| PUT | `Range: selector={css}` | Replace matching elements | 200 + full HTML |
| POST | `Range: selector={css}` | Append to matching elements | 200 + full HTML |
| DELETE | `Range: selector={css}` | Remove matching elements | 204 No Content |

### CSS Selector Examples

All valid CSS selectors should work with rust-beam. See MDN for a more complete list.

```bash
# ID selectors
Range: selector=#my-id

# Class selectors  
Range: selector=.my-class

# Attribute selectors
Range: selector=input[type="text"]

# Pseudo-selectors
Range: selector=li:first-child

# Complex selectors
Range: selector=.container > .item:nth-child(2)
```

## 🧪 Testing & Development

### Building for Release

```bash
cargo build --release
```

### Example Test HTML

Create `files/test.html`:
```html
<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body>
    <header id="main-header">
        <h1 class="title">Welcome</h1>
        <nav class="navbar">
            <ul>
                <li><a href="/">Home</a></li>
                <li><a href="/about">About</a></li>
            </ul>
        </nav>
    </header>
    <main class="content">
        <p class="intro">This is a test page.</p>
        <div class="container">
            <p>Container content</p>
        </div>
    </main>
</body>
</html>
```

### Test Commands

```bash
# Test basic file serving
curl http://127.0.0.1:3000/test.html

# Test selector operations
curl -H "Range: selector=.title" http://127.0.0.1:3000/test.html
curl -X PUT -H "Range: selector=.intro" -d '<p class="intro">Updated intro</p>' http://127.0.0.1:3000/test.html
curl -X POST -H "Range: selector=.container" -d '<p>Appended paragraph</p>' http://127.0.0.1:3000/test.html
curl -X DELETE -H "Range: selector=nav" http://127.0.0.1:3000/test.html
```
