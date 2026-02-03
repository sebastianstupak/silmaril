# Coverage analysis script for agent-game-engine (Windows PowerShell)
# Uses cargo-llvm-cov for accurate coverage reporting

param(
    [switch]$SkipInstall = $false,
    [switch]$Html = $true
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Set-Location $ProjectRoot

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Agent Game Engine - Coverage Analysis" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# Check if cargo-llvm-cov is installed
$llvmCovInstalled = $null -ne (Get-Command cargo-llvm-cov -ErrorAction SilentlyContinue)

if (-not $llvmCovInstalled -and -not $SkipInstall) {
    Write-Host "cargo-llvm-cov not found. Installing..." -ForegroundColor Yellow
    cargo install cargo-llvm-cov
}

# Clean previous coverage data
Write-Host "Cleaning previous coverage data..." -ForegroundColor White
cargo llvm-cov clean --workspace

# Run tests with coverage
Write-Host ""
Write-Host "Running tests with coverage instrumentation..." -ForegroundColor White
Write-Host "This may take a few minutes..." -ForegroundColor Yellow
Write-Host ""

# Run coverage for the entire workspace
cargo llvm-cov `
    --workspace `
    --all-features `
    --lcov `
    --output-path coverage.lcov `
    --ignore-filename-regex '(tests?|benches?|examples?)/.*\.rs$' `
    -- --test-threads=1

# Generate HTML report
if ($Html) {
    Write-Host ""
    Write-Host "Generating HTML coverage report..." -ForegroundColor White
    cargo llvm-cov report --html --output-dir coverage-html
}

# Generate summary
Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Coverage Summary" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
cargo llvm-cov report --summary-only

# Generate detailed per-module report
Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Per-Module Coverage" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
cargo llvm-cov report | Select-String -Pattern "^(engine|Filename)" | Select-Object -First 50

# Check coverage targets
Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Coverage Target Validation" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan

# Extract overall coverage percentage
$coverageOutput = cargo llvm-cov report --summary-only | Out-String
if ($coverageOutput -match "TOTAL.*?(\d+\.\d+)%") {
    $overallCoverage = [double]$Matches[1]

    Write-Host "Overall Coverage: $overallCoverage%" -ForegroundColor White
    Write-Host "Target: 80%" -ForegroundColor White

    if ($overallCoverage -ge 80.0) {
        Write-Host "✓ Overall coverage target met!" -ForegroundColor Green
    } else {
        Write-Host "⚠ Overall coverage below target" -ForegroundColor Yellow
    }
}

# Per-module targets
$moduleTargets = @{
    "engine/core" = 85
    "engine/renderer" = 80
    "engine/assets" = 85
    "engine/networking" = 80
    "engine/physics" = 80
}

Write-Host ""
Write-Host "Module-specific targets:" -ForegroundColor White
foreach ($module in $moduleTargets.Keys) {
    $target = $moduleTargets[$module]
    Write-Host "  $module`: target ${target}%" -ForegroundColor Gray
}

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Coverage Report Generated" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Reports available at:" -ForegroundColor White
Write-Host "  - LCOV: coverage.lcov" -ForegroundColor Gray
if ($Html) {
    Write-Host "  - HTML: coverage-html\index.html" -ForegroundColor Gray
}
Write-Host ""
if ($Html) {
    Write-Host "To view HTML report:" -ForegroundColor White
    Write-Host "  start coverage-html\index.html" -ForegroundColor Gray
}
Write-Host ""
