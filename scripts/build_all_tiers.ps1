# Build script for all platform-specific tiers (PowerShell version)
# Creates optimized binaries for baseline, modern, and high-end CPUs
#
# Usage:
#   .\scripts\build_all_tiers.ps1 [-Release] [-Client] [-Server] [-Both]
#
# Examples:
#   .\scripts\build_all_tiers.ps1 -Release -Both    # Build all tiers for client and server
#   .\scripts\build_all_tiers.ps1 -Client           # Build client only (debug mode)
#   .\scripts\build_all_tiers.ps1 -Release -Server  # Build server only (release mode)

param(
    [switch]$Release,
    [switch]$Client,
    [switch]$Server,
    [switch]$Both,
    [switch]$Help
)

# Show help if requested
if ($Help) {
    Write-Host "Usage: .\scripts\build_all_tiers.ps1 [-Release] [-Client] [-Server] [-Both]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Release    Build in release mode (default: debug)"
    Write-Host "  -Client     Build client binary"
    Write-Host "  -Server     Build server binary"
    Write-Host "  -Both       Build both client and server"
    Write-Host "  -Help       Show this help message"
    exit 0
}

# Set error action preference
$ErrorActionPreference = "Stop"

# Default to both if neither specified
if (-not $Client -and -not $Server) {
    $Both = $true
}

if ($Both) {
    $Client = $true
    $Server = $true
}

# Determine build mode
$Mode = if ($Release) { "release" } else { "debug" }
$ReleaseFlag = if ($Release) { "--release" } else { "" }

Write-Host "======================================" -ForegroundColor Blue
Write-Host "Building Multi-Tier Binaries" -ForegroundColor Blue
Write-Host "======================================" -ForegroundColor Blue
Write-Host "Mode: $Mode" -ForegroundColor Yellow
Write-Host "Client: $Client" -ForegroundColor Yellow
Write-Host "Server: $Server" -ForegroundColor Yellow
Write-Host ""

# Tier definitions
$Tiers = @{
    "baseline" = @{
        Name = "baseline"
        Features = "SSE2 (100% compatible)"
        RustFlags = ""
        Performance = "1.0x (baseline)"
    }
    "modern" = @{
        Name = "modern"
        Features = "AVX2+FMA+SSE4.2 (95% compatible, x86-64-v3)"
        RustFlags = "-C target-cpu=x86-64-v3"
        Performance = "1.15-1.30x (15-30% faster)"
    }
    "highend" = @{
        Name = "highend"
        Features = "AVX512+AVX2 (70% compatible, x86-64-v4)"
        RustFlags = "-C target-cpu=x86-64-v4"
        Performance = "1.20-1.50x (20-50% faster)"
    }
}

# Function to build a binary for a specific tier
function Build-Tier {
    param(
        [string]$TierName,
        [hashtable]$TierConfig,
        [string]$Binary
    )

    Write-Host "[Building] $Binary ($TierName)" -ForegroundColor Green
    Write-Host "  Features: $($TierConfig.Features)"
    Write-Host "  Expected: $($TierConfig.Performance)"

    # Create custom target directory for this tier
    $TierTargetDir = "target\$TierName"

    # Set environment variable for rustflags
    if ($TierConfig.RustFlags) {
        $env:RUSTFLAGS = $TierConfig.RustFlags
    } else {
        $env:RUSTFLAGS = ""
    }

    try {
        # Build
        $BuildArgs = @("build", "--bin", $Binary, "--target-dir", $TierTargetDir)
        if ($ReleaseFlag) {
            $BuildArgs += $ReleaseFlag
        }

        cargo @BuildArgs 2>&1 | Out-Null

        # Check if binary exists
        $BinaryPath = Join-Path $TierTargetDir "$Mode\$Binary.exe"
        if (Test-Path $BinaryPath) {
            $Size = (Get-Item $BinaryPath).Length / 1MB
            Write-Host "  ✓ Built successfully (size: $([math]::Round($Size, 2)) MB)" -ForegroundColor Green
        } else {
            Write-Host "  ✗ Build failed" -ForegroundColor Red
            return $false
        }
    } catch {
        Write-Host "  ✗ Build failed: $_" -ForegroundColor Red
        return $false
    }

    Write-Host ""
    return $true
}

# Build all tiers
Write-Host "Building tiers..." -ForegroundColor Blue
Write-Host ""

$BuildSuccess = $true

# Build client binaries
if ($Client) {
    foreach ($tier in $Tiers.Keys) {
        $result = Build-Tier -TierName $tier -TierConfig $Tiers[$tier] -Binary "client"
        if (-not $result) {
            $BuildSuccess = $false
        }
    }
}

# Build server binaries
if ($Server) {
    foreach ($tier in $Tiers.Keys) {
        $result = Build-Tier -TierName $tier -TierConfig $Tiers[$tier] -Binary "server"
        if (-not $result) {
            $BuildSuccess = $false
        }
    }
}

# Summary
Write-Host "======================================" -ForegroundColor Blue
Write-Host "Build Summary" -ForegroundColor Blue
Write-Host "======================================" -ForegroundColor Blue
Write-Host ""

if (-not $BuildSuccess) {
    Write-Host "Some builds failed. Check output above." -ForegroundColor Red
    exit 1
}

Write-Host "All builds completed successfully!" -ForegroundColor Green
Write-Host ""

# Show output locations
Write-Host "Binary locations:" -ForegroundColor Yellow
foreach ($tier in $Tiers.Keys) {
    if ($Client) {
        Write-Host "  target\$tier\$Mode\client.exe  ($($Tiers[$tier].Features))"
    }
    if ($Server) {
        Write-Host "  target\$tier\$Mode\server.exe  ($($Tiers[$tier].Features))"
    }
}
Write-Host ""

# Show runtime detection example
Write-Host @"
Runtime CPU Detection
=====================

Add this to your binary to automatically select the best tier:

``````rust
#[cfg(target_arch = "x86_64")]
fn select_binary_tier() -> &'static str {
    use std::arch::is_x86_feature_detected;

    // Check for x86-64-v4 features (AVX512)
    if is_x86_feature_detected!("avx512f") &&
       is_x86_feature_detected!("avx512dq") &&
       is_x86_feature_detected!("avx512cd") &&
       is_x86_feature_detected!("avx512bw") &&
       is_x86_feature_detected!("avx512vl") {
        return "highend";
    }

    // Check for x86-64-v3 features (AVX2, FMA)
    if is_x86_feature_detected!("avx2") &&
       is_x86_feature_detected!("fma") {
        return "modern";
    }

    // Fallback to baseline (SSE2)
    "baseline"
}
``````

"@

Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Run benchmarks: .\scripts\benchmark_tiers.ps1"
Write-Host "  2. Test on different CPUs to verify compatibility"
Write-Host "  3. Implement runtime binary selection in launcher"
Write-Host ""

Write-Host "Done!" -ForegroundColor Green
