# Rusty Beam Guestbook Demo

This demo showcases rusty-beam's DOM-aware capabilities using CSS selector-based HTTP methods.

## Features

- **Client-side DOM manipulation**: Uses DOM-aware primitives to interact with server
- **Selector-based authorization**: Different permissions for different HTML elements
- **Real-time updates**: WebSocket integration for live updates
- **No server-side logic**: All application logic lives in the client

## Running the Demo

1. Start the server:
   ```bash
   cargo run --release -- config/guestbook.html
   ```

2. Open http://localhost:3000 in your browser

## Authorization

The demo allows:
- **Anonymous users**: Can read the guestbook and add entries (no login required)
- **Admin access**: To delete entries, use HTTP Basic Auth with admin/admin123

To access admin features, use curl with authentication:
```bash
curl -u admin:admin123 -X DELETE http://localhost:3000/ \
  -H "Range: selector=#entries .entry:nth-child(1)"
```

## How It Works

The guestbook uses:
- `Range: selector=#entries` header to target specific DOM elements
- `POST` to add new entries
- `DELETE` to remove entries (admin only)
- WebSocket for real-time updates

The authorization rules ensure:
- Everyone can read entries (`GET` on `#entries`)
- Everyone can add entries (`POST` to `#entries`)
- Only administrators can delete entries (`DELETE` on `#entries .entry`)