# PowerShell script to run physics integration benchmarks
# Compares scalar vs SIMD performance across various entity counts

Write-Host "Physics Integration Benchmark Runner" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

# Navigate to physics crate
Set-Location "$PSScriptRoot\..\engine\physics"

Write-Host "Building benchmarks in release mode..." -ForegroundColor Yellow
cargo build --release --benches

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed! Exiting..." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Running benchmarks..." -ForegroundColor Yellow
Write-Host "This will take several minutes. Results will show:" -ForegroundColor Gray
Write-Host "  - Scalar vs SIMD performance comparison" -ForegroundColor Gray
Write-Host "  - Sequential vs Parallel processing" -ForegroundColor Gray
Write-Host "  - Batch size efficiency (4-wide vs 8-wide)" -ForegroundColor Gray
Write-Host ""

# Run benchmarks and save results
$timestamp = Get-Date -Format "yyyy-MM-dd_HH-mm-ss"
$resultFile = "..\..\benchmark_results_$timestamp.txt"

cargo bench --bench integration_bench 2>&1 | Tee-Object -FilePath $resultFile

Write-Host ""
Write-Host "Benchmarks complete!" -ForegroundColor Green
Write-Host "Results saved to: $resultFile" -ForegroundColor Green
Write-Host ""

# Parse results and show summary
Write-Host "Performance Summary:" -ForegroundColor Cyan
Write-Host "===================" -ForegroundColor Cyan
Get-Content $resultFile | Select-String -Pattern "time:" | Select-Object -First 20

Write-Host ""
Write-Host "Full results available in: $resultFile" -ForegroundColor Gray
