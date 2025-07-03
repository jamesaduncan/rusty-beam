# Distribution Packages

Rusty-beam is available in multiple package formats for easy installation across different platforms. Choose the package format that best suits your system and deployment needs.

## 📦 Available Packages

### Debian/Ubuntu Packages (.deb)

**Supported Systems**: Debian, Ubuntu, and derivatives

```bash
# Download and install
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam_0.1.0-1_amd64.deb
sudo dpkg -i rusty-beam_0.1.0-1_amd64.deb

# Install dependencies if needed
sudo apt-get install -f

# Start the service
rusty-beam
```

**Package Contents**:
- Binary: `/usr/bin/rusty-beam`
- Configuration: `/etc/rusty-beam/config.html`
- Plugins: `/usr/lib/rusty-beam/plugins/`
- Examples: `/usr/share/rusty-beam/examples/`
- Documentation: `/usr/share/doc/rusty-beam/`

### RPM Packages (.rpm)

**Supported Systems**: RHEL, CentOS, Fedora, SUSE

```bash
# Download and install
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-1.x86_64.rpm
sudo rpm -i rusty-beam-0.1.0-1.x86_64.rpm

# Or with yum/dnf
sudo yum install rusty-beam-0.1.0-1.x86_64.rpm
sudo dnf install rusty-beam-0.1.0-1.x86_64.rpm

# Start the service
rusty-beam
```

### Homebrew (macOS)

**Supported Systems**: macOS

```bash
# Add the tap (if using custom tap)
brew tap jamesaduncan/rusty-beam

# Install
brew install rusty-beam

# Start the service
rusty-beam

# Run as service
brew services start rusty-beam
```

### Cross-Platform Archives

**Supported Systems**: Any Linux, macOS, Windows

#### Linux (x86_64)
```bash
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-Linux-x86_64.tar.gz
tar -xzf rusty-beam-0.1.0-Linux-x86_64.tar.gz
cd rusty-beam
./rusty-beam
```

#### Linux (ARM64)
```bash
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-Linux-aarch64.tar.gz
tar -xzf rusty-beam-0.1.0-Linux-aarch64.tar.gz
cd rusty-beam
./rusty-beam
```

#### macOS (Intel)
```bash
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-macOS-x86_64.tar.gz
tar -xzf rusty-beam-0.1.0-macOS-x86_64.tar.gz
cd rusty-beam
./rusty-beam
```

#### macOS (Apple Silicon)
```bash
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-macOS-aarch64.tar.gz
tar -xzf rusty-beam-0.1.0-macOS-aarch64.tar.gz
cd rusty-beam
./rusty-beam
```

#### Windows
```powershell
# Download and extract
Invoke-WebRequest -Uri "https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-windows.zip" -OutFile "rusty-beam.zip"
Expand-Archive -Path "rusty-beam.zip" -DestinationPath "."
cd rusty-beam
.\rusty-beam.exe
```

## 🐳 Docker Images

### Official Docker Image

```bash
# Pull the latest image
docker pull ghcr.io/jamesaduncan/rusty-beam:latest

# Run with default configuration
docker run -p 3000:3000 ghcr.io/jamesaduncan/rusty-beam:latest

# Run with custom configuration
docker run -p 3000:3000 -v /path/to/config:/app/config.html ghcr.io/jamesaduncan/rusty-beam:latest

# Run with persistent data
docker run -p 3000:3000 \
  -v /path/to/config:/app/config.html \
  -v /path/to/files:/app/files \
  -v /path/to/localhost:/app/localhost \
  ghcr.io/jamesaduncan/rusty-beam:latest
```

### Docker Compose

```yaml
version: '3.8'

services:
  rusty-beam:
    image: ghcr.io/jamesaduncan/rusty-beam:latest
    ports:
      - "3000:3000"
    volumes:
      - ./config.html:/app/config.html
      - ./files:/app/files
      - ./localhost:/app/localhost
    environment:
      - RUST_LOG=info
    restart: unless-stopped
```

## 📱 AppImage (Linux)

**Portable application for any Linux distribution**

```bash
# Download
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-0.1.0-x86_64.AppImage

# Make executable
chmod +x rusty-beam-0.1.0-x86_64.AppImage

# Run
./rusty-beam-0.1.0-x86_64.AppImage
```

## 🔧 Build from Source

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install build dependencies (Ubuntu/Debian)
sudo apt-get install build-essential pkg-config libssl-dev

# Install build dependencies (macOS)
xcode-select --install

# Install build dependencies (Windows)
# Install Visual Studio Build Tools or Visual Studio Community
```

### Build Process

```bash
# Clone the repository
git clone https://github.com/jamesaduncan/rusty-beam.git
cd rusty-beam

# Build release version
cargo build --release

# Build plugins
./build-plugins.sh

# Run
./target/release/rusty-beam
```

### Create Distribution Packages

```bash
# Install packaging tools
cargo install cargo-deb cargo-generate-rpm

# Build all packages
./build-packages.sh

# Packages will be in target/packages/
ls -la target/packages/
```

## 🚀 Installation Verification

After installing, verify the installation:

```bash
# Check version
rusty-beam --version

# Check help
rusty-beam --help

# Test basic functionality
rusty-beam &
curl http://localhost:3000/
pkill rusty-beam
```

## 📋 Package Comparison

| Package Type | Pros | Cons | Best For |
|--------------|------|------|----------|
| **APT (.deb)** | System integration, automatic updates | Debian/Ubuntu only | Production Debian/Ubuntu servers |
| **RPM** | System integration, dependency management | RedHat family only | Production RHEL/CentOS/Fedora |
| **Homebrew** | Easy updates, macOS integration | macOS only | macOS development/production |
| **Tarball** | Universal, no dependencies | Manual management | Any system, containers |
| **Docker** | Isolated, reproducible | Container overhead | Containerized deployments |
| **AppImage** | Portable, no installation | Linux only, larger size | Portable Linux usage |
| **Source** | Latest features, customizable | Requires build tools | Development, customization |

## 🔄 Updates and Upgrades

### Package Manager Updates

```bash
# Debian/Ubuntu
sudo apt update && sudo apt upgrade rusty-beam

# RPM-based systems
sudo yum update rusty-beam
sudo dnf update rusty-beam

# Homebrew
brew update && brew upgrade rusty-beam
```

### Manual Updates

```bash
# Download new version
wget https://github.com/jamesaduncan/rusty-beam/releases/latest/download/rusty-beam-latest.tar.gz

# Stop current version
pkill rusty-beam

# Extract and replace
tar -xzf rusty-beam-latest.tar.gz
cp rusty-beam/rusty-beam /usr/local/bin/

# Restart
rusty-beam
```

### Docker Updates

```bash
# Pull latest image
docker pull ghcr.io/jamesaduncan/rusty-beam:latest

# Recreate container
docker-compose down
docker-compose up -d
```

## 🛠️ System Requirements

### Minimum Requirements

- **OS**: Linux (kernel 3.2+), macOS (10.12+), Windows (10+)
- **RAM**: 64 MB
- **Disk**: 50 MB for binary + content
- **CPU**: Any x86_64 or ARM64 processor

### Recommended Requirements

- **OS**: Recent Linux distribution, macOS 11+, Windows 11
- **RAM**: 256 MB
- **Disk**: 1 GB for binary + content + logs
- **CPU**: Multi-core processor for high traffic

## 🔐 Security Notes

### Package Verification

Always verify package integrity:

```bash
# Check GPG signatures (when available)
gpg --verify rusty-beam-0.1.0.tar.gz.sig rusty-beam-0.1.0.tar.gz

# Check SHA256 checksums
sha256sum rusty-beam-0.1.0.tar.gz
# Compare with published checksums
```

### Installation Security

- Download packages only from official sources
- Use package managers when possible for automatic security updates
- Run with minimal privileges (non-root user)
- Configure firewall rules appropriately

## 📞 Support

### Issues and Bug Reports

- **GitHub Issues**: https://github.com/jamesaduncan/rusty-beam/issues
- **Security Issues**: Email security@rustybeam.net

### Community

- **Discussions**: https://github.com/jamesaduncan/rusty-beam/discussions
- **Documentation**: https://docs.rustybeam.net

### Commercial Support

Contact james@stance.global for:
- Enterprise support contracts
- Custom deployment assistance
- Training and consulting
- Priority bug fixes and features

## 📄 License

Rusty-beam is released under the MIT License. See [LICENSE](https://github.com/jamesaduncan/rusty-beam/blob/main/LICENSE) for details.

All distribution packages include the same MIT license terms.