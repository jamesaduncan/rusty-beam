// Simple hello world handler using ES6 module syntax
export default function(request) {
    console.log('Hello handler received request:', request.path);
    
    return {
        status: 200,
        headers: {
            'Content-Type': 'text/plain',
            'X-Powered-By': 'Rusty-Beam-JavaScript'
        },
        body: `Hello from JavaScript ES6 module! You requested: ${request.path}`
    };
}