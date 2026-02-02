# Run delta compression benchmarks with reduced sample count for faster results
$env:CARGO_BENCH_OPTS = "--sample-size 20"

Write-Host "Running delta compression benchmarks..."
Write-Host "This may take several minutes..."

cargo bench --bench delta_benches -p engine-networking -- --sample-size 20

Write-Host "`nBenchmarks complete! Results saved in target/criterion/delta_*"
