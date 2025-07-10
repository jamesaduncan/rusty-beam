# Rusty Beam Guestbook Demo

This demo showcases rusty-beam's DOM-aware capabilities using CSS selector-based HTTP methods, now with Google OAuth2 authentication integration.

## Features

- **Client-side DOM manipulation**: Uses DOM-aware primitives to interact with server
- **Google OAuth2 Authentication**: Login with Google accounts for personalized experience
- **Selector-based authorization**: Different permissions for different HTML elements
- **Real-time updates**: WebSocket integration for live updates
- **No server-side logic**: All application logic lives in the client
- **Progressive enhancement**: Works without JavaScript, enhanced with DOM-aware features

## Running the Demo

### Option 1: With Google OAuth2 (Full Experience)

1. **Set up Google OAuth2 credentials**:
   - Go to [Google Cloud Console](https://console.cloud.google.com/)
   - Create a new project or select existing
   - Enable Google+ API
   - Create OAuth 2.0 credentials (Web application)
   - Add `http://localhost:3000/auth/google/callback` to authorized redirect URIs

2. **Configure credentials**:
   - Edit `config/guestbook.html`
   - Replace `YOUR_GOOGLE_CLIENT_ID` with your client ID
   - Replace `YOUR_GOOGLE_CLIENT_SECRET` with your client secret

3. **Run the server**:
   ```bash
   cargo run --release -- config/guestbook.html
   # Or use the helper script:
   ./run_guestbook_oauth2.sh
   ```

4. **Access the guestbook**:
   - Open http://localhost:3000 in your browser
   - Click "Login with Google" to authenticate
   - Your name will be auto-filled when posting entries

### Option 2: Without OAuth2 (Quick Testing)

For testing without setting up Google OAuth2:
```bash
cargo run --release -- config/guestbook-no-oauth.html
```

## Authorization

The demo supports multiple authentication methods:

### Google OAuth2 Users
- Can post entries with their name auto-filled
- Can be granted admin privileges by email address

### Basic Auth Users (Legacy)
- admin/admin123 - Administrator access
- guest/guest123 - Regular user access

### Anonymous Users
- Can read the guestbook
- Can post entries (must enter name manually)

## How It Works

### Authentication Flow
1. Click "Login with Google" button
2. Authenticate with Google
3. Return to guestbook with session established
4. Name field auto-populated from Google profile
5. Logout button available to end session

### DOM-Aware Features
The guestbook uses:
- `Range: selector=#entries` header to target specific DOM elements
- `POST` to add new entries
- `DELETE` to remove entries (admin only)
- `OPTIONS` for capability discovery
- `<http-can>` web component for permission checks
- WebSocket for real-time updates

### Authorization Rules
Edit `examples/guestbook/auth/authorization.html` to customize permissions:

```html
<!-- Grant admin access to a Google user -->
<tr itemscope itemtype="http://rustybeam.net/AuthorizationRule">
    <td itemprop="username">user@gmail.com</td>
    <td itemprop="path">/</td>
    <td itemprop="selector">#entries .entry</td>
    <td><ul><li itemprop="method">DELETE</li></ul></td>
    <td itemprop="action">allow</td>
</tr>
```

## Admin Features

Administrators can delete entries. Admin access can be granted to:
- Specific Google account emails
- Basic auth users with admin role

To test admin features:
```bash
# With Basic Auth
curl -u admin:admin123 -X DELETE http://localhost:3000/ \
  -H "Range: selector=#entries .entry:nth-child(1)"

# With Google OAuth2 (after granting permissions to your email)
# Use the browser interface - delete buttons will appear
```

## Security Notes

- Never commit OAuth2 credentials to version control
- Use HTTPS in production to protect session cookies
- Sessions are stored in memory (lost on server restart)
- Consider implementing session expiration for production

## Troubleshooting

### OAuth2 Issues
- **"OAuth2 configuration error"**: Check credentials in config
- **"Invalid redirect URI"**: Ensure URI matches Google Console exactly
- **Login doesn't work**: Check browser console, verify plugin order

### General Issues
- **404 errors**: Ensure all plugins are built (`./build-plugins.sh`)
- **Can't delete entries**: Check authorization rules and user permissions
- **No real-time updates**: Verify WebSocket plugin is loaded