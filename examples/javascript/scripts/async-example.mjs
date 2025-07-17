// Example demonstrating async operations with ES6 modules
export default async function(request) {
    console.log('Async handler started');
    
    // Simulate async operation with Promise
    const result = await new Promise((resolve) => {
        console.log('Starting async operation...');
        
        // setTimeout is simplified in our implementation - executes immediately
        setTimeout(() => {
            console.log('Async operation completed');
            resolve({
                message: 'This response was generated asynchronously from an ES6 module',
                timestamp: new Date().toISOString(),
                requestedPath: request.path
            });
        }, 100);
    });
    
    return {
        status: 200,
        headers: { 
            'Content-Type': 'application/json',
            'X-Async': 'true',
            'X-Module-Type': 'ES6'
        },
        body: JSON.stringify(result)
    };
}