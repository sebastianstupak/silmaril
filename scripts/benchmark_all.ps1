# Comprehensive benchmark suite for agent-game-engine (Windows PowerShell)
# Runs all benchmarks and generates performance reports

param(
    [switch]$SaveBaseline = $false,
    [switch]$CompareBaseline = $false,
    [string]$BaselineName = "baseline",
    [switch]$Quick = $false
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Set-Location $ProjectRoot

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Agent Game Engine - Comprehensive Benchmarks" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# Create benchmark results directory
$ResultsDir = Join-Path $ProjectRoot "benchmark-results"
if (-not (Test-Path $ResultsDir)) {
    New-Item -ItemType Directory -Path $ResultsDir | Out-Null
}

$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$ResultFile = Join-Path $ResultsDir "benchmark-$Timestamp.txt"

# Benchmark configuration
$BenchArgs = @()
if ($Quick) {
    $BenchArgs += "--sample-size", "10"
    Write-Host "Quick mode: Using reduced sample size" -ForegroundColor Yellow
}

# List of benchmark modules
$BenchModules = @(
    "engine/core",
    "engine/math",
    "engine/assets",
    "engine/renderer",
    "engine/networking",
    "engine/physics",
    "engine/audio",
    "engine/interest",
    "engine/auth",
    "engine/auto-update"
)

Write-Host "Starting benchmark suite at $(Get-Date)" -ForegroundColor White
Write-Host "Results will be saved to: $ResultFile" -ForegroundColor Gray
Write-Host ""

# Initialize result file
$header = @"
Agent Game Engine - Benchmark Results
======================================
Date: $(Get-Date)
System: $([System.Environment]::OSVersion.VersionString)
Rust Version: $(cargo --version)

======================================

"@
Set-Content -Path $ResultFile -Value $header

# Run benchmarks for each module
$totalModules = $BenchModules.Count
$current = 0

foreach ($module in $BenchModules) {
    $current++

    $modulePath = Join-Path $ProjectRoot $module
    if (-not (Test-Path $modulePath)) {
        Write-Host "[$current/$totalModules] Skipping $module (not found)" -ForegroundColor Yellow
        continue
    }

    # Check if module has benchmarks
    $benchDir = Join-Path $modulePath "benches"
    if (-not (Test-Path $benchDir)) {
        Write-Host "[$current/$totalModules] Skipping $module (no benchmarks)" -ForegroundColor Yellow
        continue
    }

    Write-Host "[$current/$totalModules] Running benchmarks for $module" -ForegroundColor Blue

    $moduleHeader = @"

==================================================
Module: $module
==================================================

"@
    Add-Content -Path $ResultFile -Value $moduleHeader

    # Run cargo bench for this module
    $packageName = Split-Path -Leaf $module
    try {
        $output = cargo bench --package $packageName @BenchArgs 2>&1 | Out-String
        Add-Content -Path $ResultFile -Value $output
        Write-Host "✓ Completed $module" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed $module" -ForegroundColor Red
        Add-Content -Path $ResultFile -Value "ERROR: $_"
    }

    Write-Host ""
}

# Run workspace-wide benchmarks if they exist
$workspaceBenchDir = Join-Path $ProjectRoot "benches"
if (Test-Path $workspaceBenchDir) {
    Write-Host "Running workspace-wide benchmarks" -ForegroundColor Blue

    $workspaceHeader = @"

==================================================
Workspace Benchmarks
==================================================

"@
    Add-Content -Path $ResultFile -Value $workspaceHeader

    $output = cargo bench --workspace @BenchArgs 2>&1 | Out-String
    Add-Content -Path $ResultFile -Value $output
}

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Benchmark Suite Complete" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Results saved to: $ResultFile" -ForegroundColor White

# Save baseline if requested
if ($SaveBaseline) {
    $BaselineFile = Join-Path $ResultsDir "baseline-$BaselineName.txt"
    Copy-Item $ResultFile $BaselineFile
    Write-Host "Baseline saved to: $BaselineFile" -ForegroundColor Green
}

# Compare with baseline if requested
if ($CompareBaseline) {
    $BaselineFile = Join-Path $ResultsDir "baseline-$BaselineName.txt"
    if (Test-Path $BaselineFile) {
        Write-Host ""
        Write-Host "==================================================" -ForegroundColor Cyan
        Write-Host "  Baseline Comparison" -ForegroundColor Cyan
        Write-Host "==================================================" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "Comparing with baseline: $BaselineFile" -ForegroundColor White
        Write-Host ""
        Write-Host "Note: Detailed comparison requires criterion's built-in comparison" -ForegroundColor Yellow
        Write-Host "Re-run benchmarks with -SaveBaseline to establish new baseline" -ForegroundColor Yellow
    } else {
        Write-Host "No baseline found at: $BaselineFile" -ForegroundColor Yellow
        Write-Host "Run with -SaveBaseline to create one" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "To analyze results:" -ForegroundColor White
Write-Host "  type $ResultFile" -ForegroundColor Gray
Write-Host "  # or" -ForegroundColor Gray
Write-Host "  notepad $ResultFile" -ForegroundColor Gray
Write-Host ""
