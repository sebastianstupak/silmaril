# TLS Troubleshooting Guide

Common issues and solutions for TLS implementation in silmaril.

## Quick Diagnostic Checklist

Run through this checklist first:

- [ ] Certificate files exist and are readable
- [ ] Certificate is not expired (`openssl x509 -in cert.pem -noout -dates`)
- [ ] Private key permissions are correct (0600 on Unix)
- [ ] Hostname matches certificate CN/SAN
- [ ] Firewall allows inbound connections
- [ ] System root certificates are installed
- [ ] TLS 1.3 is supported by both client and server

## Common Issues

### 1. Connection Refused

**Symptoms:**
```
Error: TLS connection error: Connection refused (os error 111)
```

**Causes:**
1. Server not running
2. Wrong port number
3. Firewall blocking connection
4. Server bound to wrong interface (localhost vs 0.0.0.0)

**Solutions:**

```bash
# Check if server is listening
netstat -tuln | grep 7777

# Check firewall (Linux)
sudo iptables -L -n | grep 7777
sudo ufw status

# Check firewall (Windows)
netsh advfirewall firewall show rule name=all | findstr 7777

# Verify server binding
# Use 0.0.0.0 for all interfaces, not 127.0.0.1
let server = TlsServer::bind("0.0.0.0:7777", config).await?;
```

### 2. Certificate Validation Failed

**Symptoms:**
```
Error: Certificate validation failed: certificate has expired
Error: Certificate validation failed: UnknownIssuer
```

**Diagnosis:**
```bash
# Check certificate expiration
openssl x509 -in cert.pem -noout -dates

# Check certificate chain
openssl verify -CAfile ca-bundle.crt fullchain.pem

# View certificate details
openssl x509 -in cert.pem -text -noout
```

**Solution 1: Certificate Expired**
```rust
// Renew certificate (Let's Encrypt)
acme.request_certificate("yourdomain.com", &cert_manager).await?;

// Or generate new self-signed (dev only)
let config = SelfSignedConfig::new("localhost").validity_days(365);
generate_and_save_self_signed_cert(&config, "cert.pem", "key.pem")?;
```

**Solution 2: Unknown Issuer (Self-Signed)**
```rust
// Development only - disable verification
let config = TlsClientConfigBuilder::new()
    .verification(CertificateVerification::Disabled)
    .build()?;
```

**Solution 3: Missing Root Certificates**
```bash
# Ubuntu/Debian
sudo apt-get install ca-certificates
sudo update-ca-certificates

# RHEL/CentOS
sudo yum install ca-certificates
sudo update-ca-trust

# Verify installation
ls /etc/ssl/certs/
```

### 3. Hostname Mismatch

**Symptoms:**
```
Error: Certificate validation failed: hostname mismatch
```

**Cause:** Certificate CN/SAN doesn't match the hostname you're connecting to

**Diagnosis:**
```bash
# Check certificate subject and SANs
openssl x509 -in cert.pem -noout -subject -ext subjectAltName

# Example output:
# subject=CN = example.com
# X509v3 Subject Alternative Name:
#     DNS:example.com, DNS:www.example.com
```

**Solutions:**

```rust
// Ensure hostname in connect() matches certificate
let conn = TlsClientConnection::connect(
    "example.com:7777",   // Must match certificate
    "example.com",        // SNI hostname must match
    config,
).await?;

// Generate certificate with correct SANs
let config = SelfSignedConfig::new("yourdomain.com")
    .add_san("yourdomain.com")
    .add_san("www.yourdomain.com")
    .add_san("192.168.1.100");  // IP addresses also supported
```

### 4. Handshake Timeout

**Symptoms:**
```
Error: TLS handshake failed: connection timeout
```

**Causes:**
1. Network latency too high
2. MTU issues (packet fragmentation)
3. Firewall inspecting TLS traffic
4. Server overloaded

**Diagnosis:**
```bash
# Test TCP connectivity
telnet yourdomain.com 7777

# Measure RTT
ping yourdomain.com

# Test with openssl
openssl s_client -connect yourdomain.com:7777 -tls1_3

# Check MTU
ip link show | grep mtu
```

**Solutions:**

```rust
// Increase timeout (if using tokio timeout)
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30),  // Longer timeout
    TlsClientConnection::connect(addr, hostname, config)
).await??;

// Check for MTU issues
// Reduce maximum message size if needed
const MAX_MESSAGE_SIZE: u32 = 1024 * 1024;  // 1MB instead of 10MB
```

### 5. Permission Denied (Private Key)

**Symptoms:**
```
Error: Failed to read private key: Permission denied (os error 13)
```

**Cause:** Incorrect file permissions on private key

**Solution:**
```bash
# Fix permissions (Unix)
chmod 600 /path/to/privkey.pem
chown gameserver:gameserver /path/to/privkey.pem

# Verify
ls -l /path/to/privkey.pem
# Should show: -rw------- 1 gameserver gameserver

# For directories
chmod 700 /path/to/certs/
```

**SELinux (if enabled):**
```bash
# Check SELinux status
getenforce

# Set correct context
sudo semanage fcontext -a -t cert_t "/etc/gameserver/certs(/.*)?"
sudo restorecon -Rv /etc/gameserver/certs

# Or disable SELinux (not recommended)
sudo setenforce 0
```

### 6. Certificate Chain Error

**Symptoms:**
```
Error: Certificate chain error: incomplete chain
```

**Cause:** Server sending leaf certificate only, not full chain

**Solution:**
```bash
# Use fullchain.pem, not cert.pem
# Correct:
let config = TlsServerConfigBuilder::new()
    .certificate("fullchain.pem", "privkey.pem")  # Includes intermediates
    .build()?;

# Wrong:
let config = TlsServerConfigBuilder::new()
    .certificate("cert.pem", "privkey.pem")  # Leaf only
    .build()?;

# Verify chain
openssl s_client -connect yourdomain.com:7777 -showcerts
```

### 7. Let's Encrypt Rate Limit

**Symptoms:**
```
Error: ACME error: too many certificates already issued for "example.com"
```

**Rate Limits:**
- 50 certificates per registered domain per week
- 5 duplicate certificates per week
- 300 new orders per account per 3 hours

**Solutions:**

```rust
// Use staging for testing
let config = AcmeConfig::new("admin@example.com", "/var/lib/acme")
    .use_staging();  // No rate limits

// Production: Wait or use different subdomains
acme.request_certificate("game1.example.com", &cert_manager).await?;
acme.request_certificate("game2.example.com", &cert_manager).await?;
```

### 8. Memory Leak / High Memory Usage

**Symptoms:**
- Server memory grows over time
- `ps aux` shows high RSS

**Diagnosis:**
```bash
# Monitor memory
watch -n 1 'ps aux | grep gameserver'

# Check session cache
# Add debug logging in code:
let stats = session_cache.stats();
println!("Sessions: {}, Memory: ~{}KB",
         stats.total_sessions,
         stats.total_sessions * 800 / 1024);
```

**Solutions:**

```rust
// Tune session cache limits
let session_cache = SessionCache::new(
    Duration::from_secs(4 * 60 * 60),  // 4 hours instead of 24
    500,  // Lower max sessions
);

// Clean up expired sessions periodically
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        session_cache.cleanup_expired();
    }
});
```

### 9. Slow Handshakes

**Symptoms:**
- Handshakes taking >100ms
- High CPU during handshakes

**Diagnosis:**
```bash
# Check CPU features
grep flags /proc/cpuinfo | grep aes

# Profile handshake
# Add profiling in code:
use std::time::Instant;
let start = Instant::now();
let conn = server.accept().await?;
println!("Handshake took: {:?}", start.elapsed());
```

**Solutions:**

1. **Enable AES-NI:**
```bash
# Verify it's available
grep aes /proc/cpuinfo

# Compile with native optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

2. **Enable session resumption:**
```rust
let config = TlsServerConfigBuilder::new()
    .enable_session_resumption(true)
    .build()?;
```

3. **Certificate chain optimization:**
```bash
# Use shorter certificate chains
# Some CAs provide alternative chains
```

### 10. Connection Reset

**Symptoms:**
```
Error: Connection reset by peer (os error 104)
```

**Causes:**
1. Client or server crashed
2. Firewall killed connection
3. TLS version mismatch
4. Cipher suite mismatch

**Diagnosis:**
```bash
# Test with openssl
openssl s_client -connect yourdomain.com:7777 -tls1_3 -showcerts

# Check server logs for errors
journalctl -u gameserver -f

# Wireshark capture
sudo tcpdump -i any -w tls_debug.pcap port 7777
```

**Solutions:**

```rust
// Ensure both sides support TLS 1.3
// Server:
let config = TlsServerConfigBuilder::new()
    .version(TlsVersion::Tls13Only)
    .build()?;

// Client:
let config = TlsClientConfigBuilder::new()
    .version(TlsVersion::Tls13Only)
    .build()?;

// Add reconnection logic
async fn connect_with_retry(addr: &str, hostname: &str, config: Arc<ClientConfig>) -> TlsResult<TlsClientConnection> {
    for attempt in 1..=3 {
        match TlsClientConnection::connect(addr, hostname, config.clone()).await {
            Ok(conn) => return Ok(conn),
            Err(e) if attempt < 3 => {
                warn!("Connection failed (attempt {}): {}", attempt, e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

## Debug Logging

Enable verbose logging to diagnose issues:

```bash
# Maximum verbosity
RUST_LOG=trace cargo run

# TLS-specific logging
RUST_LOG=engine_networking::tls=debug cargo run

# Specific module
RUST_LOG=engine_networking::tls::tcp=trace cargo run
```

In code:
```rust
use tracing::{debug, info, warn, error};

info!(
    peer_addr = %conn.peer_addr(),
    "TLS connection established"
);

debug!(
    session_id = ?session.id,
    "Session ticket created"
);

warn!(
    error = ?e,
    attempt = retry_count,
    "Handshake failed, retrying"
);
```

## Performance Issues

### High CPU Usage

**Symptoms:** Server using >50% CPU with few connections

**Diagnosis:**
```bash
# Profile the server
perf record -g ./target/release/gameserver
perf report

# Or use flamegraph
cargo install flamegraph
cargo flamegraph
```

**Solutions:**
1. Check AES-NI is enabled
2. Reduce handshake rate (use session resumption)
3. Offload to multiple cores
4. Consider hardware acceleration

### High Latency

**Symptoms:** Messages taking >100ms round-trip

**Diagnosis:**
```rust
use std::time::Instant;

// Measure send latency
let start = Instant::now();
conn.send(data).await?;
println!("Send took: {:?}", start.elapsed());

// Measure round-trip
let start = Instant::now();
conn.send(data).await?;
let response = conn.recv().await?;
println!("Round-trip: {:?}", start.elapsed());
```

**Solutions:**
1. Use connection pooling
2. Enable session resumption
3. Increase buffer sizes
4. Check network latency (not TLS issue)

## Testing Tools

### OpenSSL s_client

```bash
# Test TLS connection
openssl s_client -connect yourdomain.com:7777 -tls1_3

# Show certificate chain
openssl s_client -connect yourdomain.com:7777 -showcerts

# Test cipher suites
openssl s_client -connect yourdomain.com:7777 -cipher 'TLS_AES_256_GCM_SHA384'

# Save server certificate
openssl s_client -connect yourdomain.com:7777 -showcerts </dev/null 2>/dev/null | openssl x509 -outform PEM > server_cert.pem
```

### testssl.sh

```bash
# Install
git clone https://github.com/drwetter/testssl.sh.git

# Run comprehensive TLS test
./testssl.sh yourdomain.com:7777

# Check specific vulnerability
./testssl.sh --heartbleed yourdomain.com:7777
```

### Wireshark

```bash
# Capture TLS traffic
sudo tcpdump -i any -w tls_capture.pcap port 7777

# Open in Wireshark and filter
tls
```

## Getting Help

### Before Asking for Help

Gather this information:

1. **Error message** (full text)
2. **Rust version** (`rustc --version`)
3. **OS and version** (`uname -a` or `ver`)
4. **Certificate info** (`openssl x509 -in cert.pem -text -noout`)
5. **Minimal reproduction** (smallest code that shows issue)
6. **Logs with debug enabled** (`RUST_LOG=debug`)

### Issue Template

```markdown
## Issue Description
[Clear description of the problem]

## Environment
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75.0]
- Engine version: [e.g., 0.1.0]

## Steps to Reproduce
1. [First step]
2. [Second step]
3. [Error occurs]

## Expected Behavior
[What should happen]

## Actual Behavior
[What actually happens]

## Logs
```
[Paste relevant logs with RUST_LOG=debug]
```

## Certificates Used
[Self-signed / Let's Encrypt / Other]

## Additional Context
[Any other relevant information]
```

## References

- [TLS Setup Guide](docs/TLS_SETUP_GUIDE.md)
- [TLS Implementation Summary](TLS_IMPLEMENTATION_SUMMARY.md)
- [TLS Security Audit](TLS_SECURITY_AUDIT.md)
- [TLS Performance Report](TLS_PERFORMANCE_REPORT.md)
- [rustls Documentation](https://docs.rs/rustls/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
