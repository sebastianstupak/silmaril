#!/usr/bin/env pwsh
# Test script to verify Prometheus metrics endpoint is working
#
# Usage: .\scripts\test_prometheus_endpoint.ps1

Write-Host "Testing Prometheus Metrics Endpoint" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green
Write-Host ""

# Start server in background
Write-Host "Starting server..." -ForegroundColor Yellow
$serverJob = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    $env:RUST_LOG = "info"
    $env:METRICS_PORT = "9090"
    cargo run -p silmaril_server
}

# Wait for server to start
Write-Host "Waiting for server to start (5 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Test metrics endpoint
Write-Host ""
Write-Host "Testing metrics endpoint at http://localhost:9090/metrics" -ForegroundColor Yellow
Write-Host ""

try {
    $response = Invoke-WebRequest -Uri "http://localhost:9090/metrics" -TimeoutSec 5

    if ($response.StatusCode -eq 200) {
        Write-Host "SUCCESS: Metrics endpoint is responding!" -ForegroundColor Green
        Write-Host ""
        Write-Host "Response Preview (first 50 lines):" -ForegroundColor Cyan
        Write-Host "-----------------------------------" -ForegroundColor Cyan

        $lines = $response.Content -split "`n"
        $preview = $lines | Select-Object -First 50
        Write-Host ($preview -join "`n")

        Write-Host ""
        Write-Host "-----------------------------------" -ForegroundColor Cyan
        Write-Host "Total lines: $($lines.Count)" -ForegroundColor Cyan

        # Check for expected metrics
        Write-Host ""
        Write-Host "Checking for expected metrics..." -ForegroundColor Yellow

        $expectedMetrics = @(
            "engine_tick_duration_seconds",
            "engine_tick_rate_tps",
            "engine_entity_count",
            "engine_connected_clients",
            "engine_network_bytes_sent_total",
            "engine_network_bytes_received_total"
        )

        $found = 0
        foreach ($metric in $expectedMetrics) {
            if ($response.Content -match $metric) {
                Write-Host "  [OK] Found: $metric" -ForegroundColor Green
                $found++
            } else {
                Write-Host "  [  ] Missing: $metric" -ForegroundColor Red
            }
        }

        Write-Host ""
        if ($found -eq $expectedMetrics.Count) {
            Write-Host "ALL METRICS FOUND! ($found/$($expectedMetrics.Count))" -ForegroundColor Green
        } else {
            Write-Host "SOME METRICS MISSING ($found/$($expectedMetrics.Count))" -ForegroundColor Yellow
        }
    } else {
        Write-Host "FAILED: Got HTTP status code $($response.StatusCode)" -ForegroundColor Red
    }
} catch {
    Write-Host "FAILED: Could not connect to metrics endpoint" -ForegroundColor Red
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Red
} finally {
    # Stop server
    Write-Host ""
    Write-Host "Stopping server..." -ForegroundColor Yellow
    Stop-Job -Job $serverJob
    Remove-Job -Job $serverJob
    Write-Host "Done!" -ForegroundColor Green
}
