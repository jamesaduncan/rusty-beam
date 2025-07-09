#!/usr/bin/env node
// Simple WebSocket client to test broadcasting

const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:3000/test-broadcast.html');

ws.on('open', function open() {
  console.log('Connected to WebSocket');
  
  // Subscribe to updates
  const subscribe = {
    action: 'subscribe',
    selector: '#content',
    url: '/test-broadcast.html'
  };
  
  ws.send(JSON.stringify(subscribe));
  console.log('Sent subscription:', subscribe);
});

ws.on('message', function message(data) {
  console.log('Received:', data.toString());
  
  // Check if it's a StreamItem
  if (data.toString().includes('StreamItem')) {
    console.log('✓ Received StreamItem broadcast!');
    process.exit(0);
  }
});

ws.on('error', function error(err) {
  console.error('WebSocket error:', err);
  process.exit(1);
});

// Timeout after 30 seconds
setTimeout(() => {
  console.log('Timeout waiting for broadcast');
  process.exit(1);
}, 30000);