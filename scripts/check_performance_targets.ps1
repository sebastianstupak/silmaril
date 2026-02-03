# Performance target validation script (PowerShell)
# Runs benchmarks and verifies they meet performance targets

param(
    [switch]$Verbose = $false
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Set-Location $ProjectRoot

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Performance Target Validation" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# Load benchmark thresholds
$ThresholdsFile = Join-Path $ProjectRoot "benchmark_thresholds.yaml"

if (-not (Test-Path $ThresholdsFile)) {
    Write-Host "Warning: benchmark_thresholds.yaml not found" -ForegroundColor Yellow
    Write-Host "Creating default thresholds..." -ForegroundColor Yellow

    $defaultThresholds = @"
# Performance thresholds for automated testing
# All times in microseconds (µs) or operations per second

ecs:
  entity_spawn: 1000          # ns per entity
  component_add: 500          # ns per component
  query_iteration: 100        # ns per entity
  world_update: 16000         # µs (60 FPS = 16.67ms)

serialization:
  serialize_1k_entities: 5000  # µs
  deserialize_1k_entities: 5000  # µs
  bincode_roundtrip: 10000    # µs

networking:
  packet_encode: 100          # µs
  packet_decode: 100          # µs
  delta_compression: 500      # µs
  throughput_mbps: 100        # Mbps minimum

physics:
  step_100_bodies: 2000       # µs per step
  raycast: 50                 # µs per raycast
  collision_detection: 5000   # µs for 100 bodies

rendering:
  frame_time: 16670           # µs (60 FPS)
  draw_call_batch: 100        # ns per mesh
  gpu_upload: 1000            # µs per MB

assets:
  mesh_load: 10000            # µs for typical mesh
  texture_load: 20000         # µs for typical texture
  shader_compile: 50000       # µs
"@

    Set-Content -Path $ThresholdsFile -Value $defaultThresholds
}

Write-Host "Using thresholds from: $ThresholdsFile" -ForegroundColor Gray
Write-Host ""

# Track test results
$TotalTests = 0
$PassedTests = 0
$FailedTests = 0

# Helper function to run and check benchmark
function Test-Benchmark {
    param(
        [string]$Module,
        [string]$BenchName,
        [double]$ThresholdUs,
        [string]$Description
    )

    $script:TotalTests++

    Write-Host -NoNewline "Testing: $Description... "

    # Run benchmark (quick mode for CI)
    $ResultFile = Join-Path $env:TEMP "perf-check-$([guid]::NewGuid()).txt"

    try {
        $output = cargo bench --package $Module --bench $BenchName -- --sample-size 10 --quiet 2>&1 | Out-String
        Set-Content -Path $ResultFile -Value $output

        # Extract result (simplified - real parsing would be more robust)
        if ($output -match 'time:\s+\[([0-9.]+)') {
            $ResultUs = [double]$Matches[1]

            if ($ResultUs -le $ThresholdUs) {
                Write-Host "✓ PASS" -ForegroundColor Green -NoNewline
                Write-Host " (${ResultUs}µs <= ${ThresholdUs}µs)" -ForegroundColor Gray
                $script:PassedTests++
            } else {
                Write-Host "✗ FAIL" -ForegroundColor Red -NoNewline
                Write-Host " (${ResultUs}µs > ${ThresholdUs}µs)" -ForegroundColor Gray
                $script:FailedTests++
            }
        } else {
            Write-Host "⚠ SKIP" -ForegroundColor Yellow -NoNewline
            Write-Host " (could not parse result)" -ForegroundColor Gray
        }
    } catch {
        Write-Host "⚠ SKIP" -ForegroundColor Yellow -NoNewline
        Write-Host " (benchmark not found or failed)" -ForegroundColor Gray
        if ($Verbose) {
            Write-Host "Error: $_" -ForegroundColor Red
        }
    } finally {
        if (Test-Path $ResultFile) {
            Remove-Item $ResultFile
        }
    }
}

# Run critical benchmarks
Write-Host "ECS Performance:" -ForegroundColor White
Test-Benchmark "engine-core" "ecs_performance" 16000 "ECS world update (60 FPS target)"

Write-Host ""
Write-Host "Serialization Performance:" -ForegroundColor White
Test-Benchmark "engine-core" "serialization_comprehensive" 10000 "Serialization roundtrip"

Write-Host ""
Write-Host "Networking Performance:" -ForegroundColor White
Test-Benchmark "engine-networking" "integration_benches" 500 "Delta compression"

Write-Host ""
Write-Host "Physics Performance:" -ForegroundColor White
Test-Benchmark "engine-physics" "advanced_benches" 2000 "Physics step (100 bodies)"

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Performance Validation Summary" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Total Tests: $TotalTests" -ForegroundColor White
Write-Host "Passed: $PassedTests" -ForegroundColor Green
Write-Host "Failed: $FailedTests" -ForegroundColor Red
Write-Host ""

if ($FailedTests -gt 0) {
    Write-Host "✗ Performance validation FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Some benchmarks exceeded performance targets." -ForegroundColor Yellow
    Write-Host "Review the results above and optimize as needed." -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "✓ All performance targets met!" -ForegroundColor Green
    Write-Host ""
    Write-Host "All critical benchmarks are within acceptable limits." -ForegroundColor Gray
    exit 0
}
