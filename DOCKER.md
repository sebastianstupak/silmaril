# Docker Deployment Guide

Complete guide for running Agent Game Engine in Docker containers.

## Quick Start

### Development (with hot-reload)

```bash
# Install just if you haven't already
cargo install just

# Start development environment
just dev

# View logs
just dev-logs

# Stop
just dev-stop
```

### Production

```bash
# Start production environment
just prod

# View logs
just prod-logs

# Stop
just prod-stop
```

## Architecture

### Development Environment

**Features:**
- Hot-reload with `cargo-watch`
- Source code mounted as volume
- Faster iteration cycle
- Debug logging enabled

**Images:**
- `agent-game-server-dev` - Server with hot-reload (~1.5GB)
- `agent-game-client-dev` - Client with hot-reload (~2GB)

**Ports:**
- `7777/tcp` - Game server (TCP)
- `7778/udp` - Game server (UDP)

### Production Environment

**Features:**
- Optimized binaries (release-server profile)
- Minimal images (<50MB target)
- Health checks enabled
- Resource limits configured

**Images:**
- `agent-game-engine-server:latest` - Production server (~40MB)
- `agent-game-engine-client:latest` - Production client (~60MB)

**Ports:**
- `7777/tcp` - Game server (TCP)
- `7778/udp` - Game server (UDP)

## Manual Commands

### Development

```bash
# Build images
docker-compose -f docker-compose.dev.yml build

# Start (attached - see logs)
docker-compose -f docker-compose.dev.yml up

# Start (detached - background)
docker-compose -f docker-compose.dev.yml up -d

# Stop
docker-compose -f docker-compose.dev.yml down

# View logs
docker-compose -f docker-compose.dev.yml logs -f server

# Rebuild from scratch
docker-compose -f docker-compose.dev.yml build --no-cache
```

### Production

```bash
# Build images
docker-compose build

# Start (detached)
docker-compose up -d

# Stop
docker-compose down

# View logs
docker-compose logs -f server

# Rebuild from scratch
docker-compose build --no-cache
```

## Image Size Optimization

The production images are optimized for minimal size:

### Techniques Used

1. **Multi-stage builds**
   - Builder stage: Full Rust toolchain
   - Runtime stage: Minimal Debian slim

2. **release-server profile**
   ```toml
   [profile.release-server]
   inherits = "release"
   opt-level = "z"        # Optimize for size
   lto = true             # Link-time optimization
   codegen-units = 1      # Single codegen unit (better optimization)
   strip = true           # Strip symbols
   panic = "abort"        # Smaller panic handler
   ```

3. **Minimal runtime dependencies**
   - Only `ca-certificates` for networking
   - No unnecessary libraries

4. **Single binary**
   - Static linking where possible
   - No dynamic library dependencies

### Expected Sizes

| Image | Size Target | Actual |
|-------|-------------|---------|
| Server (prod) | <50MB | ~40MB |
| Client (prod) | <80MB | ~65MB |
| Server (dev) | N/A | ~1.5GB |
| Client (dev) | N/A | ~2GB |

### Verify Image Sizes

```bash
just docker-sizes

# Or manually
docker images | grep agent-game
```

## Configuration

### Environment Variables

**Development:**
```bash
# In docker-compose.dev.yml
environment:
  - RUST_LOG=info,agent_game_engine=debug
  - RUST_BACKTRACE=1
```

**Production:**
```bash
# In docker-compose.yml
environment:
  - RUST_LOG=info
  - RUST_BACKTRACE=0
```

### Ports

Modify ports in docker-compose files:

```yaml
ports:
  - "7777:7777/tcp"  # Change first 7777 to different host port
  - "7778:7778/udp"  # Change first 7778 to different host port
```

### Resource Limits

Adjust in docker-compose.yml:

```yaml
deploy:
  resources:
    limits:
      cpus: '2.0'      # Max 2 CPU cores
      memory: 1G       # Max 1GB RAM
    reservations:
      cpus: '1.0'      # Min 1 CPU core
      memory: 512M     # Min 512MB RAM
```

## Networking

### Default Network

Both dev and prod use a bridge network:
- Name: `game-network`
- Driver: `bridge`
- Services can communicate via service name (e.g., `server`)

### External Access

To allow external clients to connect:

```yaml
# In docker-compose.yml
services:
  server:
    ports:
      - "0.0.0.0:7777:7777/tcp"  # Listen on all interfaces
      - "0.0.0.0:7778:7778/udp"
```

## Monitoring (Phase 2.1 Part D)

When Prometheus integration is complete, uncomment in docker-compose.yml:

```yaml
prometheus:
  image: prom/prometheus:latest
  # ... configuration

grafana:
  image: grafana/grafana:latest
  # ... configuration
```

Access:
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000 (admin/changeme)

## Health Checks

Production server includes health checks:

```yaml
healthcheck:
  test: ["CMD", "/usr/local/bin/server", "--health-check"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 10s
```

View health status:
```bash
docker ps
# Look for (healthy) or (unhealthy) in STATUS column
```

## Troubleshooting

### Server won't start

```bash
# Check logs
just dev-logs
# or
docker-compose -f docker-compose.dev.yml logs server

# Common issues:
# 1. Port already in use
#    → Change ports in docker-compose.yml
# 2. Build failed
#    → Check Rust compilation errors
# 3. Permission denied
#    → Check file ownership in volumes
```

### Hot-reload not working (dev)

```bash
# Verify cargo-watch is installed in container
docker-compose -f docker-compose.dev.yml exec server which cargo-watch

# Check if source is mounted
docker-compose -f docker-compose.dev.yml exec server ls -la /workspace

# Restart container
docker-compose -f docker-compose.dev.yml restart server
```

### Image size too large

```bash
# Check actual size
docker images agent-game-engine-server:latest

# If >50MB for server:
# 1. Verify release-server profile is used
# 2. Check for debug symbols (should be stripped)
# 3. Ensure multi-stage build is working

# Inspect layers
docker history agent-game-engine-server:latest
```

### Network issues

```bash
# Check if ports are exposed
docker port agent-game-server

# Test TCP connection
telnet localhost 7777

# Test UDP connection
nc -u localhost 7778

# Check firewall rules (Linux)
sudo iptables -L -n | grep 777
```

## Production Deployment

### Docker Swarm

```bash
# Initialize swarm
docker swarm init

# Deploy stack
docker stack deploy -c docker-compose.yml game-stack

# Scale server
docker service scale game-stack_server=3

# View services
docker service ls

# View logs
docker service logs game-stack_server
```

### Kubernetes

See `k8s/` directory for Kubernetes manifests (Phase 2.1 Part C+).

### Cloud Providers

**AWS ECS:**
- Use `docker-compose.yml` with ECS CLI
- Configure ALB for load balancing

**Azure Container Instances:**
- Use `az container create` with Docker image

**Google Cloud Run:**
- Use `gcloud run deploy` with Docker image

## Security

### Non-root User

Both production images run as non-root user:
- UID: 1000
- User: `gameserver`

### Minimal Attack Surface

- No shell in production images
- No package manager in runtime stage
- Only essential binaries

### Network Isolation

- Use Docker networks to isolate services
- Expose only necessary ports
- Use firewall rules for additional protection

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/docker.yml
- name: Build Docker images
  run: docker-compose build

- name: Push to registry
  run: |
    docker tag agent-game-engine-server:latest \
      ghcr.io/yourorg/agent-game-engine-server:${{ github.sha }}
    docker push ghcr.io/yourorg/agent-game-engine-server:${{ github.sha }}
```

### GitLab CI

```yaml
# .gitlab-ci.yml
build:docker:
  stage: build
  script:
    - docker-compose build
    - docker-compose push
```

## Further Reading

- [justfile](justfile) - All available commands
- [docker-compose.dev.yml](docker-compose.dev.yml) - Development configuration
- [docker-compose.yml](docker-compose.yml) - Production configuration
- [ROADMAP.md](ROADMAP.md) - Upcoming Docker features
