   ./build.sh --arch all --name jamesaduncan774/rusty-beam --tag latest --push

    docker build -t rusty-beam:latest .

  # Tag and push to Docker Hub
  docker tag rusty-beam:latest jamesaduncan774/rusty-beam:latest
  docker push jamesduncan774/rusty-beam:latest

## REMOVE THE DEB FILE

  cd docker
  rm -f rusty-beam_*.deb
  ./build.sh --arch all --name jamesaduncan774/rusty-beam --tag latest --push
  

# Rusty-beam Docker Deployment

This directory contains everything needed to deploy rusty-beam using Docker.

## Quick Start

1. Build the Docker image:
   ```bash
   ./build.sh
   ```

2. Run with Docker Compose:
   ```bash
   docker-compose up -d
   ```

3. Or run directly with Docker:
   ```bash
   docker run -p 8080:8080 \
     -e HOSTNAME=example.com \
     -e DOCS_GIT_REPO=https://github.com/yourusername/rusty-beam-docs.git \
     rusty-beam:latest
   ```

## Directory Structure

The container sets up the following directory structure:

```
/app/                           # Server root (git repo cloned here if DOCS_GIT_REPO is set)
├── config.html                 # Server configuration file
├── localhost/public/           # Document root for localhost
├── example.com/public/         # Document root for example.com
└── www.example.com/public/     # Document root for www.example.com
```

When `DOCS_GIT_REPO` is set, the repository is cloned directly into `/app/`, making it the server root. Each hostname gets its own `public` directory that serves as the document root.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | Port to bind the server to |
| `BIND_ADDRESS` | `0.0.0.0` | Address to bind the server to |
| `CONFIG_FILE` | `` | Path to config file (relative to /app/ or absolute) |
| `HOSTNAME` | `localhost` | Primary hostname |
| `ADDITIONAL_HOSTNAMES` | `` | Comma-separated list of additional hostnames |
| `DOCS_GIT_REPO` | `` | Git repository URL to clone for docs |
| `RUST_LOG` | `info` | Logging level |
| `RUSTY_BEAM_PLUGIN_PATH` | `/usr/lib/rusty-beam/plugins` | Plugin directory path |

## Building Images

### Build for Current Architecture

```bash
./build.sh
```

### Build for Specific Architecture

```bash
# For Intel/AMD x86_64
./build.sh --arch amd64

# For ARM64
./build.sh --arch arm64
```

### Build Multi-Architecture Image

```bash
./build.sh --arch all
```

### Build and Push to Registry

```bash
./build.sh --arch all --push --name myregistry/rusty-beam --tag v1.0.0
```

```bash
cd docker
./build.sh --arch amd64
docker run -p 8080:8080 \
  -e HOSTNAME=example.com \
  -e ADDITIONAL_HOSTNAMES="www.example.com,api.example.com" \
  -e DOCS_GIT_REPO=https://github.com/yourusername/rusty-beam-docs.git \
  rusty-beam:latest

./build.sh --arch all --name jamesaduncan774/rusty-beam --tag latest --push
```

## Using a Custom Configuration File

You can store your configuration file in your git repository and reference it using the `CONFIG_FILE` environment variable:

### Example 1: Config in git repository
```bash
docker run -p 8080:8080 \
  -e DOCS_GIT_REPO=https://github.com/yourusername/rusty-beam-site.git \
  -e CONFIG_FILE="config/production.html" \
  rusty-beam:latest
```

This will use the file at `/app/config/production.html` after cloning the repository.

### Example 2: Absolute path
```bash
docker run -p 8080:8080 \
  -v ./my-config.html:/app/custom-config.html:ro \
  -e CONFIG_FILE="/app/custom-config.html" \
  rusty-beam:latest
```

### Example 3: In docker-compose.yml
```yaml
services:
  rusty-beam:
    image: rusty-beam:latest
    environment:
      DOCS_GIT_REPO: https://github.com/yourusername/site-content.git
      CONFIG_FILE: config/server.html
      HOSTNAME: example.com
```

## Git Repository Structure

When using `DOCS_GIT_REPO`, the repository is cloned directly into `/app/` as the server root. The repository should be structured with subdirectories for each hostname:

```
your-site-repo/
├── config.html                 # Optional: Server configuration
├── localhost/
│   └── public/
│       ├── index.html
│       └── assets/
├── example.com/
│   └── public/
│       ├── index.html
│       └── assets/
└── www.example.com/
    └── public/
        ├── index.html
        └── assets/
```

Each hostname directory should contain a `public` subdirectory that serves as the document root for that hostname.

## Docker Compose Example

The provided `docker-compose.yml` includes:

- Environment variable configuration
- Volume mounts for persistence
- Health checks
- Resource limits
- Restart policies

### Using Local Files

For development, you can mount local directories:

```yaml
volumes:
  - ./my-docs:/app/docs:ro
  - ./my-config.html:/app/config.html:ro
```

### Using Git Repository

Set the `DOCS_GIT_REPO` environment variable:

```yaml
environment:
  DOCS_GIT_REPO: https://github.com/yourusername/rusty-beam-docs.git
```

## Advanced Configuration

### Custom Plugins

To add custom plugins, create a custom Dockerfile:

```dockerfile
FROM rusty-beam:latest

# Copy custom plugins
COPY my-plugin.so /usr/lib/rusty-beam/plugins/
```

### SSL/TLS Termination

Use a reverse proxy like nginx or traefik for SSL termination:

```yaml
services:
  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./certs:/etc/nginx/certs:ro
    depends_on:
      - rusty-beam
```

### Scaling

To run multiple instances with different configurations:

```yaml
services:
  site1:
    extends:
      service: rusty-beam
    environment:
      HOSTNAME: site1.com
      PORT: 8081
    ports:
      - "8081:8081"
  
  site2:
    extends:
      service: rusty-beam
    environment:
      HOSTNAME: site2.com
      PORT: 8082
    ports:
      - "8082:8082"
```

## Troubleshooting

### Check Logs

```bash
docker-compose logs -f rusty-beam
```

### Verify Configuration

```bash
docker exec rusty-beam cat /app/config.html
```

### Test Health Check

```bash
docker exec rusty-beam curl -f http://localhost:8080/
```

### Rebuild After Changes

```bash
docker-compose build --no-cache
docker-compose up -d
```

## Security Notes

- The container runs as non-root user `rusty-beam`
- Only necessary ports are exposed
- Git cloning happens at runtime, so credentials can be passed via environment
- Consider using Docker secrets for sensitive configuration

## Support

For issues and feature requests, please visit:
https://github.com/jamesaduncan/rusty-beam