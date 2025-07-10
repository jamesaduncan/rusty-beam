# Environment Variables

Rusty Beam uses environment variables for sensitive configuration like API keys and secrets to follow security best practices.

## Required Environment Variables

### Google OAuth2 Plugin

The Google OAuth2 plugin requires these environment variables:

- `GOOGLE_CLIENT_ID` - Your Google OAuth2 client ID
- `GOOGLE_CLIENT_SECRET` - Your Google OAuth2 client secret

## Setting Up Environment Variables

### Local Development

1. Copy the example file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your actual values:
   ```bash
   GOOGLE_CLIENT_ID=123456789.apps.googleusercontent.com
   GOOGLE_CLIENT_SECRET=GOCSPX-xxxxxxxxxxxxxxxxxxxx
   ```

3. The `.env` file is automatically excluded from version control.

### Production Deployment

Set environment variables through your deployment platform:

#### Docker
```bash
docker run -e GOOGLE_CLIENT_ID=your_id -e GOOGLE_CLIENT_SECRET=your_secret rusty-beam
```

#### Systemd Service
```ini
[Service]
Environment=GOOGLE_CLIENT_ID=your_id
Environment=GOOGLE_CLIENT_SECRET=your_secret
```

#### Cloud Platforms
- **Heroku**: Use `heroku config:set`
- **AWS**: Use Systems Manager Parameter Store or Secrets Manager
- **Google Cloud**: Use Secret Manager
- **Azure**: Use Key Vault

## Security Best Practices

1. **Never commit secrets** to version control
2. **Use different credentials** for development/staging/production
3. **Rotate credentials regularly**
4. **Use proper secrets management** in production
5. **Limit credential permissions** to minimum required scope

## Troubleshooting

If OAuth2 isn't working, check:

1. Environment variables are set correctly
2. Google Cloud Console credentials match
3. Redirect URIs are configured properly
4. Server logs for specific error messages

The server will log warnings if required environment variables are missing.