# benchmark_all_platforms.ps1 - Cross-platform benchmark automation (Windows)
#
# Usage:
#   .\scripts\benchmark_all_platforms.ps1 [OPTIONS]
#
# Options:
#   -Baseline NAME     Save results as baseline with given name (default: current timestamp)
#   -Compare NAME      Compare with named baseline
#   -OutputDir DIR     Output directory for results (default: benchmarks/results)
#   -Quick             Run subset of benchmarks (faster)
#   -NoPlatform        Skip platform-specific benchmarks
#   -NoEcs             Skip ECS benchmarks
#   -Verbose           Enable verbose output
#   -Help              Show this help message

param(
    [string]$Baseline = "",
    [string]$Compare = "",
    [string]$OutputDir = "benchmarks/results",
    [switch]$Quick = $false,
    [switch]$NoPlatform = $false,
    [switch]$NoEcs = $false,
    [switch]$Verbose = $false,
    [switch]$Help = $false
)

# Show help if requested
if ($Help) {
    Get-Content $PSCommandPath | Select-Object -First 14 | Select-Object -Skip 1 | ForEach-Object { $_ -replace '^# ', '' }
    exit 0
}

# Colors for output
function Write-Color {
    param([string]$Color, [string]$Text)
    Write-Host $Text -ForegroundColor $Color
}

# Configuration
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$Platform = "windows"

Write-Color Cyan "=== Cross-Platform Benchmark Suite ==="
Write-Color Green "Platform: $Platform"
Write-Color Green "Timestamp: $Timestamp"
Write-Host ""

# Create output directory
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$ResultDir = Join-Path $OutputDir "${Platform}_${Timestamp}"
New-Item -ItemType Directory -Force -Path $ResultDir | Out-Null

Write-Color Green "Results will be saved to: $ResultDir"
Write-Host ""

# Verbose logging
function Write-Verbose-Log {
    param([string]$Message)
    if ($Verbose) {
        Write-Color Blue "[VERBOSE] $Message"
    }
}

# Run a benchmark suite
function Run-Benchmark {
    param(
        [string]$Name,
        [string]$Package,
        [string]$Bench
    )

    $OutputFile = Join-Path $ResultDir "${Name}.log"

    Write-Color Yellow "Running benchmark: $Name"
    Write-Verbose-Log "Package: $Package, Bench: $Bench"

    if ($Quick) {
        # Quick mode: fewer samples
        cargo bench --package $Package --bench $Bench -- `
            --warm-up-time 1 --measurement-time 3 --sample-size 20 `
            --save-baseline $Name 2>&1 | Tee-Object -FilePath $OutputFile
    } else {
        # Full mode: default Criterion settings
        cargo bench --package $Package --bench $Bench -- `
            --save-baseline $Name 2>&1 | Tee-Object -FilePath $OutputFile
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Color Red "✗ Failed: $Name"
        return $false
    }

    Write-Color Green "✓ Completed: $Name"
    Write-Host ""
    return $true
}

$Success = $true

# Platform-specific benchmarks
if (-not $NoPlatform) {
    Write-Color Cyan "=== Platform Abstraction Benchmarks ==="
    Write-Host ""

    # Time backend benchmarks
    if (-not (Run-Benchmark "platform_time" "engine-core" "platform_benches")) {
        $Success = $false
    }
}

# ECS benchmarks
if (-not $NoEcs) {
    Write-Color Cyan "=== ECS Benchmarks ==="
    Write-Host ""

    # Core ECS operations
    if (-not (Run-Benchmark "ecs_world" "engine-core" "world_benches")) { $Success = $false }
    if (-not (Run-Benchmark "ecs_query" "engine-core" "query_benches")) { $Success = $false }
    if (-not (Run-Benchmark "ecs_entity" "engine-core" "entity_benches")) { $Success = $false }

    # Storage benchmarks
    if (-not (Run-Benchmark "ecs_sparse_set" "engine-core" "sparse_set_benches")) { $Success = $false }

    # Comprehensive ECS benchmarks
    if (-not $Quick) {
        if (-not (Run-Benchmark "ecs_comprehensive" "engine-core" "ecs_comprehensive_benches")) {
            $Success = $false
        }
    }
}

# Physics benchmarks
Write-Color Cyan "=== Physics Benchmarks ==="
Write-Host ""
if (-not (Run-Benchmark "physics_integration" "engine-physics" "integration_bench")) {
    $Success = $false
}

# Math/SIMD benchmarks
Write-Color Cyan "=== Math/SIMD Benchmarks ==="
Write-Host ""
if (-not (Run-Benchmark "math_simd" "engine-math" "simd_benches")) { $Success = $false }
if (-not (Run-Benchmark "math_transform" "engine-math" "transform_benches")) { $Success = $false }

# Serialization benchmarks
Write-Color Cyan "=== Serialization Benchmarks ==="
Write-Host ""
if (-not (Run-Benchmark "serialization" "engine-core" "serialization_benches")) {
    $Success = $false
}

# Profiling overhead benchmarks
Write-Color Cyan "=== Profiling Overhead Benchmarks ==="
Write-Host ""
if (-not (Run-Benchmark "profiling_overhead" "engine-profiling" "profiling_overhead")) {
    $Success = $false
}

# Generate summary report
Write-Host ""
Write-Color Cyan "=== Generating Summary Report ==="
Write-Host ""

$SummaryFile = Join-Path $ResultDir "SUMMARY.md"
$QuickMode = if ($Quick) { "Quick" } else { "Full" }
$GitCommit = (git rev-parse HEAD 2>$null) ?? "unknown"
$GitBranch = (git rev-parse --abbrev-ref HEAD 2>$null) ?? "unknown"

$Summary = @"
# Benchmark Results Summary

**Platform:** $Platform
**Date:** $(Get-Date)
**Mode:** $QuickMode

---

## Benchmark Suites Run

"@

if (-not $NoPlatform) {
    $Summary += @"

### Platform Abstraction
- Time Backend

"@
}

if (-not $NoEcs) {
    $Summary += @"

### ECS
- World Operations
- Query System
- Entity Management
- Sparse Set Storage

"@
    if (-not $Quick) {
        $Summary += "- Comprehensive ECS`n"
    }
}

$Summary += @"

### Physics
- Integration System

### Math/SIMD
- SIMD Operations
- Transform Operations

### Serialization
- Component Serialization

### Profiling
- Profiling Overhead

---

## Files Generated

- Benchmark logs: ``*.log``
- Criterion output: ``target/criterion/``
- Summary: ``SUMMARY.md``

---

## Next Steps

1. **View detailed results:**
   ``````powershell
   Start-Process target/criterion/report/index.html
   ``````

2. **Compare with baseline:**
   ``````powershell
   .\scripts\benchmark_all_platforms.ps1 -Compare baseline_name
   ``````

3. **Check for regressions:**
   ``````powershell
   python scripts/compare_with_industry.py --results $ResultDir
   ``````

"@

Set-Content -Path $SummaryFile -Value $Summary
Write-Color Green "Summary report saved to: $SummaryFile"

# Save as baseline if requested
if ($Baseline -ne "") {
    Write-Host ""
    Write-Color Cyan "=== Saving Baseline ==="
    Write-Host ""

    $BaselineDir = "benchmarks/baselines/${Platform}_${Baseline}"
    New-Item -ItemType Directory -Force -Path $BaselineDir | Out-Null

    # Copy Criterion baselines
    Copy-Item -Path "target/criterion" -Destination $BaselineDir -Recurse -Force

    # Copy our results
    Copy-Item -Path $ResultDir -Destination "$BaselineDir/results" -Recurse -Force

    # Save metadata
    $Metadata = @{
        platform = $Platform
        baseline_name = $Baseline
        timestamp = $Timestamp
        quick_mode = $Quick.IsPresent
        git_commit = $GitCommit
        git_branch = $GitBranch
    } | ConvertTo-Json

    Set-Content -Path "$BaselineDir/metadata.json" -Value $Metadata

    Write-Color Green "✓ Baseline saved to: $BaselineDir"
}

# Compare with baseline if requested
if ($Compare -ne "") {
    Write-Host ""
    Write-Color Cyan "=== Comparing with Baseline ==="
    Write-Host ""

    $BaselineDir = "benchmarks/baselines/${Platform}_${Compare}"

    if (-not (Test-Path $BaselineDir)) {
        Write-Color Red "Error: Baseline '$Compare' not found for platform '$Platform'"
        Write-Host "Available baselines:"
        Get-ChildItem "benchmarks/baselines" -Filter "${Platform}_*" | ForEach-Object {
            $name = $_.Name -replace "^${Platform}_", ""
            Write-Host "  - $name"
        }
        exit 1
    }

    Write-Color Green "Baseline: $Compare"
    Write-Host "Using Python comparison script..."
    Write-Host ""

    python scripts/benchmark_regression_check.py `
        --baseline "$BaselineDir/criterion" `
        --current "target/criterion" `
        --threshold 20 `
        --format criterion `
        --output "$ResultDir/comparison.md"

    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Color Green "✓ Comparison report saved to: $ResultDir/comparison.md"
    } else {
        Write-Color Red "✗ Comparison failed"
        $Success = $false
    }
}

# Final summary
Write-Host ""
Write-Color Green "=== Benchmark Suite Complete ==="
Write-Host ""
Write-Color Blue "Results directory: $ResultDir"
Write-Color Blue "HTML report: target/criterion/report/index.html"
Write-Host ""
Write-Host "To view HTML report:"
Write-Color Yellow "  Start-Process target/criterion/report/index.html"
Write-Host ""

if (-not $Success) {
    Write-Color Red "Some benchmarks failed. Check logs for details."
    exit 1
}

exit 0
