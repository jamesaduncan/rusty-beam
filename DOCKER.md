# Docker Deployment Guide for Rusty Beam

This guide explains how to build and run Rusty Beam using Docker.

## Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-org/rusty-beam.git
   cd rusty-beam
   ```

2. **Create environment file**
   Create a `.env` file with your OAuth2 credentials:
   ```bash
   # Google OAuth2
   GOOGLE_CLIENT_ID=your-google-client-id
   GOOGLE_CLIENT_SECRET=your-google-client-secret
   GOOGLE_OAUTH2_CALLBACK=https://yourdomain.com/auth/google/callback

   # GitHub OAuth2 (optional)
   GITHUB_CLIENT_ID=your-github-client-id
   GITHUB_CLIENT_SECRET=your-github-client-secret
   GITHUB_OAUTH2_CALLBACK=https://yourdomain.com/auth/github/callback
   ```

3. **Build and run**
   ```bash
   docker-compose up --build
   ```

## Configuration

The Docker setup automatically:
- Binds to `0.0.0.0` instead of `127.0.0.1` for container accessibility
- Uses port 3000 by default (configurable via `PORT` environment variable)
- Configures OAuth2 endpoints based on your environment variables
- Serves the web content from `/app/docs`

## OAuth2 Configuration

The config file (`/docs/config/index.html`) includes two OAuth2 providers by default:

1. **Google OAuth2**
   - Login path: `/auth/google/login`
   - Callback path: Derived from `GOOGLE_OAUTH2_CALLBACK` environment variable
   - Required env vars: `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_OAUTH2_CALLBACK`

2. **GitHub OAuth2**
   - Login path: `/auth/github/login`
   - Callback path: Derived from `GITHUB_OAUTH2_CALLBACK` environment variable
   - Required env vars: `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_OAUTH2_CALLBACK`

## Railway.com Deployment

For Railway.com deployment, the Docker setup automatically:
- Detects and uses the `PORT` environment variable
- Configures the hostname from `RAILWAY_PUBLIC_DOMAIN`
- Suggests OAuth2 callback URLs based on the Railway domain

## Custom Configuration

To use a custom configuration file instead of the default:

1. Mount your config file:
   ```yaml
   volumes:
     - ./my-config.html:/app/docs/config/index.html
   ```

2. Or build a custom image with your config:
   ```dockerfile
   FROM rusty-beam:latest
   COPY my-config.html /app/docs/config/index.html
   ```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Server port | 3000 |
| `GOOGLE_CLIENT_ID` | Google OAuth2 client ID | Required |
| `GOOGLE_CLIENT_SECRET` | Google OAuth2 client secret | Required |
| `GOOGLE_OAUTH2_CALLBACK` | Google OAuth2 callback URL | Required |
| `GITHUB_CLIENT_ID` | GitHub OAuth2 client ID | Optional |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth2 client secret | Optional |
| `GITHUB_OAUTH2_CALLBACK` | GitHub OAuth2 callback URL | Optional |
| `RAILWAY_PUBLIC_DOMAIN` | Railway.com public domain | Auto-detected |

## Testing

Run the minimal test to verify your Docker setup:
```bash
./test-docker-minimal.sh
```

## Troubleshooting

1. **Container exits immediately**
   - Check logs: `docker-compose logs`
   - Ensure all required environment variables are set
   - Verify the config file is valid HTML

2. **OAuth2 not working**
   - Verify environment variables are set correctly
   - Check that callback URLs match your OAuth2 provider configuration
   - Ensure the server hostname is accessible from the internet

3. **Port conflicts**
   - Change the port mapping in `docker-compose.yml`
   - Or set the `PORT` environment variable

## Production Considerations

For production deployment:

1. Use secrets management for OAuth2 credentials
2. Enable HTTPS (use a reverse proxy like nginx)
3. Set resource limits in docker-compose.yml
4. Use a persistent volume for session data if needed
5. Configure proper logging and monitoring