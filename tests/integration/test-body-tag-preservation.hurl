# Test that body tags are preserved when using PUT with CSS selectors

# Start by getting the original content
GET http://{{host}}:{{port}}/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Captures]
original_content: body

# PUT request to update the body content using CSS selector
PUT http://{{host}}:{{port}}/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
Range: selector=html:nth-child(1) > body:nth-child(2)
Content-Type: text/html
```
<body>
    <h1>Updated Title</h1>
    <ul>
        <li>One</li>
        <li>Two</li>
        <li>Three</li>
    </ul>
</body>
```
HTTP 200

# Verify the body tags are preserved in the response
GET http://{{host}}:{{port}}/
Authorization: Basic YWRtaW46YWRtaW4xMjM=
HTTP 200
[Asserts]
body contains "<body>"
body contains "</body>"
body contains "<h1>Updated Title</h1>"