# WebSocket Plugin for Rusty Beam

Real-time bidirectional communication plugin that enables WebSocket connections to Rusty Beam documents.

## Features

- WebSocket protocol (RFC 6455) implementation
- Real-time broadcasting of document changes
- CSS selector-based subscriptions
- StreamItem microdata format for updates
- Integration with selector-handler plugin
- Multiple concurrent connections support

## Installation

The plugin is built as part of the standard Rusty Beam plugin suite:

```bash
cd plugins/websocket
cargo build --release
```

## Configuration

Add to your Rusty Beam configuration:

```html
<table itemscope itemtype="http://rustybeam.net/Plugin">
    <tbody>
        <tr>
            <td>Library</td>
            <td itemprop="library">file://./plugins/librusty_beam_websocket.so</td>
        </tr>
    </tbody>
</table>
```

## Usage

### JavaScript Client

```javascript
const ws = new WebSocket('ws://localhost:3000/document.html');

ws.onopen = () => {
    // Subscribe to changes
    ws.send(JSON.stringify({
        action: 'subscribe',
        selector: '#content',
        url: '/document.html'
    }));
};

ws.onmessage = (event) => {
    console.log('Update:', event.data);
    // Receives StreamItem format updates
};
```

### Triggering Updates

Updates are broadcast when content is modified via selector-handler:

```bash
curl -X PUT \
  -H "Range: selector=#content" \
  -d '<div id="content">New content</div>' \
  http://localhost:3000/document.html
```

## Protocol

### Client Messages

Subscribe:
```json
{
    "action": "subscribe",
    "selector": "#content",
    "url": "/document.html"
}
```

Unsubscribe:
```json
{
    "action": "unsubscribe",
    "selector": "#content"
}
```

### Server Messages

StreamItem broadcast:
```html
<div itemscope itemtype="http://rustybeam.net/StreamItem">
    <span itemprop="method">PUT</span>
    <span itemprop="url">/document.html</span>
    <span itemprop="selector">#content</span>
    <div itemprop="content">
        <div id="content">New content</div>
    </div>
</div>
```

## Integration

The WebSocket plugin integrates with:
- **selector-handler**: Detects changes and triggers broadcasts
- **authorization**: Respects document access rules
- **access-log**: Logs upgrade requests (101 status)

## Testing

Run the WebSocket tests:

```bash
cargo test --test websocket-tests
cargo test --test websocket-broadcast-tests
```

## Future Enhancements

- Connection limits and rate limiting
- Ping/pong interval configuration
- Message size limits
- Compression support
- Binary message support for efficiency

## License

Same as Rusty Beam - Apache License 2.0