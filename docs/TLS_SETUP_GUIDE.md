# TLS Setup Guide

Complete guide for setting up TLS encryption in the agent-game-engine.

## Table of Contents

- [Quick Start](#quick-start)
- [Certificate Management](#certificate-management)
- [Production Deployment](#production-deployment)
- [Let's Encrypt Automation](#lets-encrypt-automation)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Development Setup (Self-Signed Certificates)

For local development and testing, use self-signed certificates:

```rust
use engine_networking::tls::certificates::{SelfSignedConfig, generate_and_save_self_signed_cert};

// Generate certificate
let config = SelfSignedConfig::new("localhost")
    .add_san("127.0.0.1")
    .add_san("::1")
    .validity_days(365);

generate_and_save_self_signed_cert(&config, "dev_cert.pem", "dev_key.pem")?;
```

### Server Configuration

```rust
use engine_networking::tls::{TlsServer, TlsServerConfigBuilder};

// Configure server with certificate
let config = TlsServerConfigBuilder::new()
    .certificate("dev_cert.pem", "dev_key.pem")
    .build()?;

// Create TLS server
let server = TlsServer::bind("0.0.0.0:7777", config).await?;

// Accept connections
loop {
    let mut conn = server.accept().await?;

    tokio::spawn(async move {
        // Handle connection
        while let Ok(msg) = conn.recv().await {
            conn.send(&msg).await.ok();
        }
    });
}
```

### Client Configuration

```rust
use engine_networking::tls::{TlsClientConnection, TlsClientConfigBuilder, CertificateVerification};

// Development: Disable verification for self-signed certs
let config = TlsClientConfigBuilder::new()
    .verification(CertificateVerification::Disabled)
    .build()?;

// Connect to server
let mut conn = TlsClientConnection::connect(
    "127.0.0.1:7777",
    "localhost",
    config,
).await?;

// Send encrypted data
conn.send(b"Hello, encrypted world!").await?;
let response = conn.recv().await?;
```

## Certificate Management

### Self-Signed Certificates (Development Only)

**Use Case:** Local development, internal testing

**Pros:**
- No external dependencies
- Free
- Instant generation

**Cons:**
- Not trusted by browsers/clients by default
- No protection against man-in-the-middle attacks
- Not suitable for production

**Generation:**

```rust
use engine_networking::tls::certificates::{SelfSignedConfig, generate_and_save_self_signed_cert};

let config = SelfSignedConfig::new("myserver.local")
    .add_san("192.168.1.100")
    .organization("My Company")
    .organizational_unit("Development")
    .validity_days(365);

generate_and_save_self_signed_cert(&config, "cert.pem", "key.pem")?;
```

### CA-Signed Certificates (Production)

**Use Case:** Production deployments, public-facing servers

**Options:**
1. **Let's Encrypt** (Recommended) - Free, automated
2. **Commercial CA** - DigiCert, Sectigo, etc.
3. **Internal CA** - For enterprise environments

**Manual Setup with Let's Encrypt:**

```bash
# Install certbot
sudo apt-get install certbot

# Request certificate (HTTP-01 challenge)
sudo certbot certonly --standalone -d yourdomain.com -d www.yourdomain.com

# Certificates will be in /etc/letsencrypt/live/yourdomain.com/
```

**Use in Server:**

```rust
let config = TlsServerConfigBuilder::new()
    .certificate(
        "/etc/letsencrypt/live/yourdomain.com/fullchain.pem",
        "/etc/letsencrypt/live/yourdomain.com/privkey.pem"
    )
    .build()?;
```

### Certificate Formats

The engine accepts PEM format certificates:

**Certificate File (cert.pem):**
```
-----BEGIN CERTIFICATE-----
MIIFazCCA1OgAwIBAgIRAIIQz7DSQONZRGPgu2OCiwAwDQYJKoZIhvcNAQELBQAw
...
-----END CERTIFICATE-----
```

**Private Key File (key.pem):**
```
-----BEGIN PRIVATE KEY-----
MIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDU8Meh0k6Pf2zb
...
-----END PRIVATE KEY-----
```

**Convert from other formats:**

```bash
# Convert DER to PEM
openssl x509 -inform DER -in cert.der -out cert.pem

# Convert PKCS#12 to PEM
openssl pkcs12 -in cert.p12 -out cert.pem -nodes
openssl pkcs12 -in cert.p12 -out key.pem -nodes -nocerts
```

## Production Deployment

### Security Checklist

- [ ] Use CA-signed certificates (never self-signed)
- [ ] Enable certificate validation on clients
- [ ] Set restrictive file permissions on private keys (0600)
- [ ] Store private keys securely (consider HSM for high-security)
- [ ] Enable automated certificate renewal
- [ ] Monitor certificate expiration dates
- [ ] Use TLS 1.3 only (no fallback to older versions)
- [ ] Use strong cipher suites only
- [ ] Enable Perfect Forward Secrecy
- [ ] Set up certificate pinning for critical services
- [ ] Log all TLS handshake failures
- [ ] Monitor for weak cipher attempts

### File Permissions

```bash
# Certificate (public) - readable by all
chmod 644 /path/to/cert.pem

# Private key - readable by owner only
chmod 600 /path/to/key.pem
chown gameserver:gameserver /path/to/key.pem

# Directory permissions
chmod 700 /path/to/certs/
```

### Certificate Storage

**Development:**
```
./certs/
├── dev_cert.pem
└── dev_key.pem
```

**Production (Linux):**
```
/etc/gameserver/certs/
├── fullchain.pem  (certificate + intermediate chain)
└── privkey.pem    (private key)

# Or use Let's Encrypt default location
/etc/letsencrypt/live/yourdomain.com/
├── fullchain.pem
├── privkey.pem
├── cert.pem       (certificate only)
└── chain.pem      (intermediate chain)
```

### Server Configuration

```rust
use engine_networking::tls::{TlsServer, TlsServerConfigBuilder};

let config = TlsServerConfigBuilder::new()
    .certificate(
        "/etc/gameserver/certs/fullchain.pem",
        "/etc/gameserver/certs/privkey.pem"
    )
    // Optional: Require client authentication (mutual TLS)
    .require_client_auth("/etc/gameserver/certs/client_ca.pem")
    // Enable session resumption for performance
    .enable_session_resumption(true)
    .build()?;

let server = TlsServer::bind("0.0.0.0:7777", config).await?;
```

### Client Configuration

```rust
use engine_networking::tls::{TlsClientConnection, TlsClientConfigBuilder};

// Production: Full verification with system roots
let config = TlsClientConfigBuilder::new()
    .build()?;  // Uses CertificateVerification::Full by default

let mut conn = TlsClientConnection::connect(
    "game.example.com:7777",
    "game.example.com",  // Must match certificate CN/SAN
    config,
).await?;
```

### Monitoring and Alerting

Track these metrics:

```rust
use tracing::{info, warn};

// Log successful connections
info!(
    peer_addr = %conn.peer_addr(),
    "TLS connection established"
);

// Alert on handshake failures
warn!(
    error = ?e,
    "TLS handshake failed"
);

// Monitor certificate expiration
if cert_info.days_until_expiration < Some(30) {
    warn!(
        domain = %cert_info.subject,
        days = ?cert_info.days_until_expiration,
        "Certificate expiring soon"
    );
}
```

## Let's Encrypt Automation

### Automated Certificate Acquisition

```rust
use engine_networking::tls::certificates::{AcmeClient, AcmeConfig, CertificateManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure ACME client
    let config = AcmeConfig::new("admin@example.com", "/var/lib/acme");
    let mut acme = AcmeClient::new(config);

    // Initialize (creates account if needed)
    acme.initialize().await?;

    // Create certificate manager
    let cert_manager = CertificateManager::new("/etc/gameserver/certs")?;

    // Request certificate
    acme.request_certificate("game.example.com", &cert_manager).await?;

    Ok(())
}
```

### HTTP-01 Challenge Requirements

For Let's Encrypt HTTP-01 challenge, you need:

1. **Web server** serving on port 80
2. **File accessible** at `http://yourdomain.com/.well-known/acme-challenge/{token}`
3. **Firewall rules** allowing inbound port 80

**Simple HTTP server for ACME challenges:**

```rust
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn run_acme_challenge_server() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:80").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            socket.read(&mut buffer).await.ok();

            // Parse request and serve challenge file
            let request = String::from_utf8_lossy(&buffer);
            if request.contains("/.well-known/acme-challenge/") {
                // Serve challenge file
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 87\r\n\r\n{challenge_response}";
                socket.write_all(response.as_bytes()).await.ok();
            }
        });
    }
}
```

### Automated Renewal

Set up a renewal task:

```rust
use tokio::time::{interval, Duration};

async fn renewal_task(
    acme: &mut AcmeClient,
    cert_manager: &CertificateManager,
) {
    let mut interval = interval(Duration::from_secs(24 * 60 * 60)); // Daily check

    loop {
        interval.tick().await;

        match acme.renew_certificates(cert_manager).await {
            Ok(renewed) => {
                if !renewed.is_empty() {
                    info!("Renewed certificates: {:?}", renewed);
                    // Reload server with new certificates
                }
            }
            Err(e) => {
                warn!("Certificate renewal failed: {}", e);
            }
        }
    }
}
```

### Systemd Timer (Linux)

Create `/etc/systemd/system/gameserver-certrenew.service`:

```ini
[Unit]
Description=Game Server Certificate Renewal
After=network-online.target

[Service]
Type=oneshot
User=gameserver
ExecStart=/usr/local/bin/gameserver-certrenew
```

Create `/etc/systemd/system/gameserver-certrenew.timer`:

```ini
[Unit]
Description=Game Server Certificate Renewal Timer

[Timer]
OnCalendar=daily
RandomizedDelaySec=3600
Persistent=true

[Install]
WantedBy=timers.target
```

Enable:

```bash
sudo systemctl enable gameserver-certrenew.timer
sudo systemctl start gameserver-certrenew.timer
```

## Troubleshooting

### Certificate Validation Fails

**Symptoms:** Client can't connect, "certificate validation failed" error

**Solutions:**

1. **Check certificate expiration:**
```bash
openssl x509 -in cert.pem -noout -dates
```

2. **Verify hostname matches:**
```bash
openssl x509 -in cert.pem -noout -subject -ext subjectAltName
```

3. **Check certificate chain:**
```bash
openssl verify -CAfile ca-bundle.crt fullchain.pem
```

4. **System root certificates:**
```bash
# Ubuntu/Debian
sudo apt-get install ca-certificates
sudo update-ca-certificates

# RHEL/CentOS
sudo yum install ca-certificates
sudo update-ca-trust
```

### Handshake Timeout

**Symptoms:** Connection times out during TLS handshake

**Solutions:**

1. **Check network connectivity:**
```bash
telnet game.example.com 7777
```

2. **Verify firewall rules:**
```bash
sudo iptables -L -n | grep 7777
```

3. **Check TLS version support:**
```bash
openssl s_client -connect game.example.com:7777 -tls1_3
```

### Permission Denied (Private Key)

**Symptoms:** "Permission denied" when loading private key

**Solutions:**

1. **Fix file permissions:**
```bash
chmod 600 /path/to/privkey.pem
chown gameserver:gameserver /path/to/privkey.pem
```

2. **Check SELinux (if enabled):**
```bash
sudo semanage fcontext -a -t cert_t "/etc/gameserver/certs(/.*)?"
sudo restorecon -Rv /etc/gameserver/certs
```

### Let's Encrypt Rate Limits

**Symptoms:** "too many certificates already issued" error

**Solutions:**

1. **Use staging environment for testing:**
```rust
let config = AcmeConfig::new("admin@example.com", "/var/lib/acme")
    .use_staging();
```

2. **Check rate limits:**
- 50 certificates per domain per week
- 5 duplicate certificates per week
- Use staging for development/testing

3. **Share certificates** across multiple servers using certificate storage

### Connection Reset During Handshake

**Symptoms:** Connection resets immediately after connecting

**Solutions:**

1. **Check TLS version compatibility:**
   - Ensure both client and server support TLS 1.3
   - Check cipher suite compatibility

2. **Verify certificate is valid:**
```bash
openssl s_client -connect game.example.com:7777 -showcerts
```

3. **Check server logs** for handshake errors

### Memory/Performance Issues

**Symptoms:** High memory usage, slow handshakes

**Solutions:**

1. **Enable session resumption:**
```rust
.enable_session_resumption(true)
```

2. **Tune connection limits:**
```rust
// Set maximum concurrent connections
const MAX_CONNECTIONS: usize = 10000;
```

3. **Monitor hardware acceleration:**
```bash
# Check if AES-NI is available
grep -q aes /proc/cpuinfo && echo "AES-NI available" || echo "AES-NI not available"
```

4. **Profile memory usage:**
```rust
use engine_profiling::profile_scope;

#[profile(category = "TLS")]
async fn handle_connection(conn: TlsServerConnection) {
    // Memory will be tracked
}
```

## Additional Resources

- [TLS Implementation Summary](TLS_IMPLEMENTATION_SUMMARY.md)
- [TLS Security Audit](TLS_SECURITY_AUDIT.md)
- [TLS Performance Report](TLS_PERFORMANCE_REPORT.md)
- [Certificate Management Guide](CERTIFICATE_MANAGEMENT.md)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [Rustls Documentation](https://docs.rs/rustls/)
