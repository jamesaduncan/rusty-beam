# Docker Deployment Guide for Rusty Beam

## Quick Start

### Local Development
```bash
# Build and run locally
docker-compose up --build

# Access the server
curl http://localhost:3000/demos/guestbook/index.html
```

### Production Deployment

The Docker setup is optimized for Railway.com deployment with the following features:

1. **Automatic Configuration Updates**
   - Bind address automatically changed from 127.0.0.1 to 0.0.0.0
   - Port dynamically set from `$PORT` environment variable
   - Railway hostname automatically detected and configured

2. **OAuth Support**
   - Set `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` environment variables
   - These are automatically passed to the container

3. **Build Optimization**
   - Uses Cargo workspace for faster builds
   - All plugins built in a single pass
   - Multi-stage Dockerfile reduces final image size

## Railway Deployment

1. Push to your GitHub repository
2. Connect Railway to your GitHub repo
3. Set environment variables in Railway:
   ```
   GOOGLE_CLIENT_ID=your_client_id
   GOOGLE_CLIENT_SECRET=your_client_secret
   ```
4. Railway will automatically:
   - Set `PORT` environment variable
   - Set `RAILWAY_PUBLIC_DOMAIN` with your app URL
   - Build and deploy using the Dockerfile

## Docker Images

### Standard Build
```bash
docker build -t rusty-beam:latest .
```

### Optimized Build (with cargo-chef)
```bash
docker build -f Dockerfile.optimized -t rusty-beam:optimized .
```

## Environment Variables

- `PORT` - Server port (default: 3000)
- `RAILWAY_PUBLIC_DOMAIN` - Railway hostname (auto-detected)
- `RAILWAY_STATIC_URL` - Alternative Railway hostname
- `GOOGLE_CLIENT_ID` - Google OAuth client ID
- `GOOGLE_CLIENT_SECRET` - Google OAuth client secret

## Testing Deployment

Use the included test script:
```bash
./test-railway-deployment.sh
```

## Configuration

The server uses `/app/docs/config/index.html` as its configuration file. The docker-entrypoint.sh script automatically:
- Updates bind address for Docker compatibility
- Sets the port from environment variables
- Adds Railway hostnames when deployed

## Troubleshooting

1. **Container exits immediately**: The server runs in verbose mode (-v) to prevent daemonization
2. **Can't access server**: Ensure bind address is 0.0.0.0, not 127.0.0.1
3. **OAuth not working**: Check environment variables are set correctly
4. **Railway build timeout**: Use the workspace-optimized Dockerfile