# Network Benchmark Runner (PowerShell)
#
# Quick script to run network integration benchmarks with common options

param(
    [Parameter(Position=0)]
    [string]$Mode = "quick",

    [Parameter(Position=1)]
    [string]$Baseline = ""
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $ProjectRoot

function Write-Header {
    param([string]$Message)
    Write-Host "========================================" -ForegroundColor Green
    Write-Host $Message -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO]" -ForegroundColor Yellow -NoNewline
    Write-Host " $Message"
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR]" -ForegroundColor Red -NoNewline
    Write-Host " $Message"
}

switch ($Mode) {
    "quick" {
        Write-Header "Running Quick Network Benchmarks"
        Write-Info "Running: end_to_end_latency and simulator_overhead"
        cargo bench --bench integration_benches -- --quick 'end_to_end_latency|simulator_overhead'
    }

    "full" {
        Write-Header "Running Full Network Benchmark Suite"
        Write-Info "This will take 10-20 minutes..."
        cargo bench --bench integration_benches
    }

    "scenarios" {
        Write-Header "Running Game Scenario Benchmarks"
        cargo bench --bench integration_benches -- game_scenarios
    }

    "latency" {
        Write-Header "Running Latency Benchmarks"
        cargo bench --bench integration_benches -- end_to_end_latency
    }

    "bandwidth" {
        Write-Header "Running Bandwidth Benchmarks"
        cargo bench --bench integration_benches -- bandwidth_usage
    }

    "scalability" {
        Write-Header "Running Scalability Benchmarks"
        cargo bench --bench integration_benches -- 'concurrent_clients|scalability'
    }

    "resilience" {
        Write-Header "Running Packet Loss Resilience Benchmarks"
        cargo bench --bench integration_benches -- packet_loss_resilience
    }

    "baseline" {
        if ([string]::IsNullOrEmpty($Baseline)) {
            $Baseline = "main"
        }
        Write-Header "Creating Baseline: $Baseline"
        cargo bench --bench integration_benches -- --save-baseline $Baseline
        Write-Info "Baseline saved as '$Baseline'"
    }

    "compare" {
        if ([string]::IsNullOrEmpty($Baseline)) {
            Write-Error "Baseline name required for comparison"
            Write-Host "Usage: .\run_network_benchmarks.ps1 compare <baseline-name>"
            exit 1
        }
        Write-Header "Comparing Against Baseline: $Baseline"
        cargo bench --bench integration_benches -- --baseline $Baseline
    }

    "report" {
        Write-Header "Opening Benchmark Report"
        $ReportPath = Join-Path $ProjectRoot "target\criterion\report\index.html"
        if (Test-Path $ReportPath) {
            Write-Info "Opening $ReportPath"
            Start-Process $ReportPath
        } else {
            Write-Error "Report not found. Run benchmarks first."
            exit 1
        }
    }

    "clean" {
        Write-Header "Cleaning Benchmark Data"
        $CriterionPath = Join-Path $ProjectRoot "target\criterion"
        if (Test-Path $CriterionPath) {
            Remove-Item -Recurse -Force $CriterionPath
        }
        Write-Info "Benchmark data cleaned"
    }

    { $_ -in "help", "--help", "-h" } {
        Write-Host @"
Network Benchmark Runner

Usage: .\run_network_benchmarks.ps1 [mode] [baseline-name]

Modes:
    quick           Run quick tests (latency + overhead) [default]
    full            Run complete benchmark suite (10-20 min)
    scenarios       Run game scenario benchmarks (MMORPG, FPS, etc.)
    latency         Run end-to-end latency benchmarks
    bandwidth       Run bandwidth usage benchmarks
    scalability     Run concurrent client and scalability benchmarks
    resilience      Run packet loss resilience benchmarks

    baseline <name> Create a new baseline for comparison
    compare <name>  Compare current performance vs baseline
    report          Open HTML benchmark report in browser
    clean           Remove all benchmark data
    help            Show this help message

Examples:
    .\run_network_benchmarks.ps1 quick                  # Quick test
    .\run_network_benchmarks.ps1 full                   # Full suite
    .\run_network_benchmarks.ps1 baseline main          # Save baseline
    .\run_network_benchmarks.ps1 compare main           # Compare
    .\run_network_benchmarks.ps1 latency                # Only latency
    .\run_network_benchmarks.ps1 report                 # View results

Output:
    Results saved to: target\criterion\
    HTML report: target\criterion\report\index.html
"@
    }

    default {
        Write-Error "Unknown mode: $Mode"
        Write-Host "Run '.\run_network_benchmarks.ps1 help' for usage information"
        exit 1
    }
}

if ($Mode -notin @("report", "clean", "help", "--help", "-h")) {
    Write-Host ""
    Write-Info "Benchmark complete!"
    Write-Info "View results: .\run_network_benchmarks.ps1 report"
    Write-Info "View HTML report: target\criterion\report\index.html"
}
