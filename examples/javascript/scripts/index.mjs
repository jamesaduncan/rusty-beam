// Default index handler using ES6 module syntax
export default function(request) {
    const html = `
<!DOCTYPE html>
<html>
<head>
    <title>Rusty Beam JavaScript Engine - ES6 Modules</title>
    <style>
        body { font-family: sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        h1 { color: #333; }
        .endpoint { background: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 5px; }
        code { background: #e8e8e8; padding: 2px 4px; border-radius: 3px; }
        .info { background: #e3f2fd; border: 1px solid #90caf9; padding: 15px; border-radius: 5px; margin: 20px 0; }
    </style>
</head>
<body>
    <h1>Welcome to Rusty Beam JavaScript Engine!</h1>
    <div class="info">
        <strong>ES6 Module Support:</strong> This page is served by an ES6 module using the default export syntax.
    </div>
    
    <h2>Available Endpoints:</h2>
    <div class="endpoint">
        <strong>GET /hello</strong> - Simple text response
    </div>
    <div class="endpoint">
        <strong>GET /api/users</strong> - JSON list of users
    </div>
    <div class="endpoint">
        <strong>GET /api/status</strong> - Server status
    </div>
    <div class="endpoint">
        <strong>GET /api/echo</strong> - Echo request details
    </div>
    <div class="endpoint">
        <strong>GET /async</strong> - Async operation example
    </div>
    
    <h2>ES6 Module Example:</h2>
    <pre><code>// hello.mjs
export default function(request) {
    return {
        status: 200,
        headers: { 'Content-Type': 'text/plain' },
        body: \`Hello from \${request.path}\`
    };
}</code></pre>
    
    <h2>Request Details:</h2>
    <pre>${JSON.stringify({
        method: request.method,
        path: request.path,
        headers: request.headers
    }, null, 2)}</pre>
</body>
</html>`;
    
    return {
        status: 200,
        headers: { 'Content-Type': 'text/html; charset=utf-8' },
        body: html
    };
}