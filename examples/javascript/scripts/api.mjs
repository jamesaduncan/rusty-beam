// API handler with JSON responses using ES6 module syntax
async function handleApiRequest(request) {
    console.log('API handler:', request.method, request.path);
    
    // Parse the path to determine the API endpoint
    const pathParts = request.path.split('/').filter(p => p);
    const endpoint = pathParts[1]; // After 'api'
    
    switch (endpoint) {
        case 'users':
            return handleUsers(request);
        case 'status':
            return handleStatus(request);
        case 'echo':
            return handleEcho(request);
        default:
            return {
                status: 404,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ error: 'Endpoint not found' })
            };
    }
}

function handleUsers(request) {
    const users = [
        { id: 1, name: 'Alice', email: 'alice@example.com' },
        { id: 2, name: 'Bob', email: 'bob@example.com' },
        { id: 3, name: 'Charlie', email: 'charlie@example.com' }
    ];
    
    return {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(users)
    };
}

function handleStatus(request) {
    return {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            status: 'ok',
            timestamp: new Date().toISOString(),
            version: '1.0.0'
        })
    };
}

function handleEcho(request) {
    return {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            method: request.method,
            path: request.path,
            headers: request.headers,
            body: request.body
        })
    };
}

// ES6 default export
export default handleApiRequest;