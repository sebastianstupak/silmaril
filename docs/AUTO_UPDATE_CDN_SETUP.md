# Auto-Update CDN Setup Guide

## Overview

This guide provides step-by-step instructions for setting up a Content Delivery Network (CDN) infrastructure for the auto-update system. We'll use AWS S3 + CloudFront as the primary example, with alternative options included.

## Architecture

```
[Game Client]
    ↓ HTTPS
[CloudFront CDN]
    ↓ Origin Request
[S3 Bucket]
    ├── manifest.json (index)
    ├── stable/
    │   ├── 1.0.0/manifest.json
    │   ├── 1.0.0/game.exe
    │   ├── 1.0.0/assets/...
    │   ├── 1.0.1/manifest.json
    │   └── 1.0.1/patches/game.exe.patch
    ├── beta/
    │   └── [same structure]
    └── dev/
        └── [same structure]
```

## Option 1: AWS S3 + CloudFront (Recommended)

### Prerequisites

- AWS Account
- AWS CLI installed and configured
- Domain name (optional but recommended)

### Step 1: Create S3 Bucket

```bash
# Set your bucket name (must be globally unique)
BUCKET_NAME="your-game-updates"
REGION="us-east-1"

# Create bucket
aws s3 mb s3://${BUCKET_NAME} --region ${REGION}

# Enable versioning (for backup/rollback)
aws s3api put-bucket-versioning \
    --bucket ${BUCKET_NAME} \
    --versioning-configuration Status=Enabled

# Configure bucket policy for CloudFront access
cat > bucket-policy.json <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowCloudFrontAccess",
      "Effect": "Allow",
      "Principal": {
        "Service": "cloudfront.amazonaws.com"
      },
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::${BUCKET_NAME}/*",
      "Condition": {
        "StringEquals": {
          "AWS:SourceArn": "arn:aws:cloudfront::ACCOUNT_ID:distribution/DISTRIBUTION_ID"
        }
      }
    }
  ]
}
EOF

# Apply policy (after CloudFront setup)
aws s3api put-bucket-policy --bucket ${BUCKET_NAME} --policy file://bucket-policy.json
```

### Step 2: Upload Initial Structure

```bash
# Create directory structure
mkdir -p updates/{stable,beta,dev}

# Create manifest index
cat > updates/manifest.json <<EOF
{
  "channels": {
    "stable": "1.0.0",
    "beta": "1.0.1",
    "dev": "1.1.0"
  },
  "manifest_url_template": "https://cdn.yourgame.com/{channel}/{version}/manifest.json"
}
EOF

# Upload to S3
aws s3 sync updates/ s3://${BUCKET_NAME}/ --acl private
```

### Step 3: Create CloudFront Distribution

```bash
# Get S3 bucket domain name
S3_DOMAIN="${BUCKET_NAME}.s3.${REGION}.amazonaws.com"

# Create CloudFront distribution
aws cloudfront create-distribution \
    --distribution-config file://cloudfront-config.json
```

**cloudfront-config.json:**
```json
{
  "CallerReference": "update-cdn-$(date +%s)",
  "Comment": "Game Update CDN",
  "Enabled": true,
  "Origins": {
    "Quantity": 1,
    "Items": [
      {
        "Id": "S3-game-updates",
        "DomainName": "your-game-updates.s3.us-east-1.amazonaws.com",
        "OriginAccessControlId": "E1234567890ABC",
        "S3OriginConfig": {
          "OriginAccessIdentity": ""
        }
      }
    ]
  },
  "DefaultCacheBehavior": {
    "TargetOriginId": "S3-game-updates",
    "ViewerProtocolPolicy": "https-only",
    "AllowedMethods": {
      "Quantity": 2,
      "Items": ["GET", "HEAD"],
      "CachedMethods": {
        "Quantity": 2,
        "Items": ["GET", "HEAD"]
      }
    },
    "MinTTL": 0,
    "DefaultTTL": 86400,
    "MaxTTL": 31536000,
    "Compress": true,
    "ForwardedValues": {
      "QueryString": false,
      "Cookies": {
        "Forward": "none"
      }
    }
  },
  "PriceClass": "PriceClass_All",
  "ViewerCertificate": {
    "CloudFrontDefaultCertificate": true,
    "MinimumProtocolVersion": "TLSv1.2_2021"
  }
}
```

### Step 4: Configure Custom Domain (Optional)

```bash
# Request ACM certificate (must be in us-east-1 for CloudFront)
aws acm request-certificate \
    --domain-name updates.yourgame.com \
    --validation-method DNS \
    --region us-east-1

# Wait for validation...
# Then add CNAME record to Route53 or your DNS provider
```

### Step 5: Configure Cache Behaviors

```bash
# Update distribution for optimal caching
aws cloudfront update-distribution \
    --id E1234567890ABC \
    --distribution-config file://cloudfront-cache-config.json
```

**Recommended Cache Settings:**
- **Manifest files**: 5 minutes (short TTL for updates)
- **Game files**: 1 year (immutable, versioned URLs)
- **Patches**: 1 year (immutable)

### Step 6: Enable Logging

```bash
# Create logging bucket
aws s3 mb s3://${BUCKET_NAME}-logs

# Enable CloudFront logging
aws cloudfront update-distribution \
    --id E1234567890ABC \
    --logging-config Enabled=true,Bucket=${BUCKET_NAME}-logs.s3.amazonaws.com,Prefix=cloudfront/
```

## Option 2: Cloudflare R2 + CDN

### Advantages
- Cheaper egress costs ($0 vs AWS's $0.085/GB)
- Built-in CDN (no separate service)
- Simpler setup

### Setup Steps

```bash
# Install Wrangler CLI
npm install -g wrangler

# Authenticate
wrangler login

# Create R2 bucket
wrangler r2 bucket create game-updates

# Upload files
wrangler r2 object put game-updates/manifest.json --file=updates/manifest.json

# Enable public access with custom domain
wrangler r2 bucket expose game-updates --domain updates.yourgame.com
```

### Cost Comparison (for 10TB/month)

| Service | S3 + CloudFront | Cloudflare R2 |
|---------|----------------|---------------|
| Storage (1TB) | $23 | $15 |
| Requests (10M) | $10 | $4.50 |
| Egress (10TB) | $850 | $0 |
| **Total** | **$883/month** | **$19.50/month** |

## Option 3: Self-Hosted (nginx)

For full control or regulatory requirements:

### nginx Configuration

```nginx
server {
    listen 443 ssl http2;
    server_name updates.yourgame.com;

    ssl_certificate /etc/letsencrypt/live/updates.yourgame.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/updates.yourgame.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;

    root /var/www/game-updates;

    # Cache manifest for 5 minutes
    location ~* manifest\.json$ {
        add_header Cache-Control "public, max-age=300";
        add_header X-Content-Type-Options "nosniff";
    }

    # Cache game files for 1 year (versioned URLs)
    location ~* \.(exe|dll|pak|dat)$ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    # Enable compression
    gzip on;
    gzip_types application/json application/octet-stream;
    gzip_min_length 1024;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=updates:10m rate=10r/s;
    limit_req zone=updates burst=20 nodelay;

    # Logging
    access_log /var/log/nginx/updates-access.log combined;
    error_log /var/log/nginx/updates-error.log warn;
}
```

## Security Best Practices

### 1. HTTPS Only

```python
# In UpdateManager::check_for_updates()
if manifest_url.starts_with("http://") {
    return Err(UpdateError::checkfailed("HTTPS required".to_string()));
}
```

### 2. Certificate Pinning (Optional)

For high-security games:

```rust
use reqwest::tls;

let client = Client::builder()
    .tls_built_in_root_certs(false)
    .add_root_certificate(Certificate::from_pem(CA_CERT)?)
    .build()?;
```

### 3. Manifest Signing

```bash
# Generate Ed25519 key pair
python3 generate_keys.py

# Sign manifest
python3 sign_manifest.py --manifest=stable/1.0.1/manifest.json --key=private.key

# Public key goes in game config
```

**generate_keys.py:**
```python
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization

# Generate key pair
private_key = ed25519.Ed25519PrivateKey.generate()
public_key = private_key.public_key()

# Save keys
with open('private.key', 'wb') as f:
    f.write(private_key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.PKCS8,
        encryption_algorithm=serialization.NoEncryption()
    ))

with open('public.key', 'wb') as f:
    f.write(public_key.public_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PublicFormat.SubjectPublicKeyInfo
    ))

print(f"Public key (hex): {public_key.public_bytes_raw().hex()}")
```

## Deployment Workflow

### CI/CD Pipeline (GitHub Actions)

```yaml
name: Deploy Update

on:
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build game
        run: cargo build --release

      - name: Generate patches
        run: |
          cargo run --bin generate_patch -- \
            --old-dir ./previous-version \
            --new-dir ./target/release \
            --output ./updates \
            --version ${{ github.event.release.tag_name }} \
            --channel stable

      - name: Sign manifest
        run: python3 scripts/sign_manifest.py --manifest=updates/manifest.json

      - name: Upload to S3
        run: |
          aws s3 sync updates/ s3://game-updates/stable/${{ github.event.release.tag_name }}/

      - name: Invalidate CloudFront cache
        run: |
          aws cloudfront create-invalidation \
            --distribution-id ${{ secrets.CLOUDFRONT_DIST_ID }} \
            --paths "/manifest.json" "/stable/*"

      - name: Update manifest index
        run: |
          python3 scripts/update_manifest_index.py \
            --channel stable \
            --version ${{ github.event.release.tag_name }}
```

## Monitoring & Alerts

### CloudWatch Alarms

```bash
# Monitor 4xx/5xx error rates
aws cloudwatch put-metric-alarm \
    --alarm-name game-updates-errors \
    --alarm-description "High error rate on update CDN" \
    --metric-name 4xxErrorRate \
    --namespace AWS/CloudFront \
    --statistic Average \
    --period 300 \
    --threshold 5.0 \
    --comparison-operator GreaterThanThreshold \
    --evaluation-periods 2

# Monitor bandwidth
aws cloudwatch put-metric-alarm \
    --alarm-name game-updates-bandwidth \
    --metric-name BytesDownloaded \
    --namespace AWS/CloudFront \
    --statistic Sum \
    --period 3600 \
    --threshold 10000000000 \
    --comparison-operator GreaterThanThreshold
```

### Custom Metrics

Track in your game server:
- Update success rate
- Average download time
- Rollback rate
- Version distribution

## Cost Optimization

### 1. Regional Caching

```python
# Use regional endpoints in manifest
"manifest_url_template": "https://cdn-{region}.yourgame.com/{channel}/{version}/manifest.json"

# Detect region in client
region = detect_optimal_region()  # Based on latency
url = manifest_url_template.replace("{region}", region)
```

### 2. Delta Compression

```bash
# Use Zstd dictionaries for better compression
zstd --train patches/*.patch -o patch.dict
zstd -D patch.dict new.patch -o new.patch.zst
```

### 3. Smart Invalidation

```bash
# Only invalidate changed paths
aws cloudfront create-invalidation \
    --distribution-id E1234567890ABC \
    --paths "/manifest.json" "/stable/1.0.1/*"
```

## Testing

### Load Testing

```bash
# Install k6
brew install k6

# Run load test
k6 run loadtest.js
```

**loadtest.js:**
```javascript
import http from 'k6/http';
import { check } from 'k6';

export let options = {
  stages: [
    { duration: '1m', target: 100 },  // Ramp up
    { duration: '5m', target: 100 },  // Sustained load
    { duration: '1m', target: 0 },    // Ramp down
  ],
};

export default function () {
  let res = http.get('https://updates.yourgame.com/manifest.json');
  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });
}
```

## Disaster Recovery

### 1. Multi-Region Failover

```json
{
  "manifest_url_template": "https://cdn.yourgame.com/{channel}/{version}/manifest.json",
  "fallback_urls": [
    "https://cdn-eu.yourgame.com/{channel}/{version}/manifest.json",
    "https://cdn-ap.yourgame.com/{channel}/{version}/manifest.json"
  ]
}
```

### 2. Rollback Procedure

```bash
# Revert to previous version
aws s3 sync s3://game-updates-backup/stable/1.0.0/ s3://game-updates/stable/1.0.1/
aws cloudfront create-invalidation --distribution-id E1234567890ABC --paths "/*"
```

### 3. Emergency Kill Switch

```json
{
  "channels": {
    "stable": "1.0.0",  // Revert to known-good version
    "beta": null,       // Disable beta updates
    "dev": null         // Disable dev updates
  }
}
```

## Conclusion

Recommended setup for most games:
- **Small studios (<10K players)**: Cloudflare R2 ($20-50/month)
- **Medium studios (<100K players)**: AWS S3 + CloudFront ($100-500/month)
- **Large studios (>100K players)**: Multi-CDN with failover ($1000+/month)

Always test your CDN setup before launch and monitor costs/performance closely.
