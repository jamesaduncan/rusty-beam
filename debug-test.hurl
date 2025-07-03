# Debug test for CSS selector DELETE
# Test only the problematic sequence

# Upload test.html file
PUT http://127.0.0.1:3000/test.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Test</title></head>
<body>
    <h1>Test Title</h1>
    <p>Test paragraph</p>
    <div>Test div to be deleted</div>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test CSS selector DELETE - Remove specific element
DELETE http://127.0.0.1:3000/test.html
Range: selector=div
HTTP 204

# Check if file still exists after element deletion
GET http://127.0.0.1:3000/test.html
HTTP 200
[Asserts]
body contains "Test Title"
body contains "Test paragraph"

# Upload complex.html file
PUT http://127.0.0.1:3000/complex.html
Content-Type: text/html
```
<!DOCTYPE html>
<html>
<head><title>Complex Test</title></head>
<body>
    <nav>
        <ul class="menu">
            <li><a href="/">Home</a></li>
            <li><a href="/about">About</a></li>
            <li><a href="/contact">Contact</a></li>
        </ul>
    </nav>
    <main>
        <article id="main-content">
            <h1>Main Article</h1>
            <p class="intro">Introduction paragraph</p>
            <p>Regular paragraph</p>
        </article>
        <aside>
            <h2>Sidebar</h2>
            <p>Sidebar content</p>
        </aside>
    </main>
</body>
</html>
```
HTTP 201
[Asserts]
body == "File uploaded successfully"

# Test ID selector
GET http://127.0.0.1:3000/complex.html
Range: selector=#main-content
HTTP 200
[Asserts]
body contains "Main Article"
body contains "Introduction paragraph"