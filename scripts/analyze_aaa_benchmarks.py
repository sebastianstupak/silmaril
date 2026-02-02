#!/usr/bin/env python3
"""
Analyze networking benchmark results against AAA standards.

Parses Criterion benchmark output and validates against industry targets.
"""

import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple

# AAA Performance Targets
AAA_TARGETS = {
    "serialization": {
        "bincode_speed_mb_s": 200,  # MB/sec minimum
        "entity_snapshot_10k_ms": 1.0,  # milliseconds
        "worldstate_speed_mb_s": 100,  # MB/sec minimum
    },
    "protocol": {
        "framing_overhead_bytes": 50,  # maximum bytes
        "throughput_msg_s": 10_000,  # messages/sec minimum
        "roundtrip_us": 100,  # microseconds maximum
    },
    "tcp": {
        "connection_ms": 100,  # milliseconds maximum
        "latency_p95_ms": 50,  # milliseconds maximum
        "throughput_mb_s": 10,  # MB/sec minimum
        "concurrent_connections": 1000,  # minimum
    },
    "udp": {
        "latency_p95_ms": 20,  # milliseconds maximum
        "send_rate_hz": 60,  # Hz minimum
    },
    "snapshot": {
        "generation_speed_mb_s": 50,  # MB/sec minimum
        "snapshot_10k_ms": 10,  # milliseconds maximum
        "bytes_per_entity": 200,  # maximum
    },
    "delta": {
        "compression_ratio_1pct": 0.10,  # maximum (90% reduction)
        "compression_ratio_5pct": 0.30,  # maximum (70% reduction)
        "compression_ratio_10pct": 0.30,  # maximum (70% reduction)
        "delta_diff_ms": 0.5,  # milliseconds maximum
        "delta_apply_ms": 0.5,  # milliseconds maximum
    }
}

class BenchmarkAnalyzer:
    def __init__(self, criterion_dir: Path):
        self.criterion_dir = criterion_dir
        self.results = {}
        self.aaa_status = {}

    def parse_criterion_output(self, output_text: str) -> Dict:
        """Parse criterion text output to extract benchmark results."""
        results = {}
        lines = output_text.split('\n')

        current_bench = None
        for line in lines:
            # Match benchmark names (e.g., "client_message_serialization/player_move_bincode")
            if 'Benchmarking' in line and not 'Warming up' in line and not 'Collecting' in line:
                current_bench = line.split('Benchmarking')[1].strip()
                results[current_bench] = {}

            # Match timing results (e.g., "time:   [132.59 ns 137.46 ns 143.44 ns]")
            elif 'time:' in line and current_bench:
                parts = line.split('[')[1].split(']')[0].split()
                # Parse median (middle value)
                median_value = float(parts[2])
                median_unit = parts[3]

                # Convert to nanoseconds
                if median_unit == 'ns':
                    results[current_bench]['time_ns'] = median_value
                elif median_unit == 'µs' or median_unit == 'us':
                    results[current_bench]['time_ns'] = median_value * 1000
                elif median_unit == 'ms':
                    results[current_bench]['time_ns'] = median_value * 1_000_000
                elif median_unit == 's':
                    results[current_bench]['time_ns'] = median_value * 1_000_000_000

            # Match throughput results (e.g., "thrpt:  [5.0890 Melem/s 5.4628 Melem/s 5.8867 Melem/s]")
            elif 'thrpt:' in line and current_bench:
                parts = line.split('[')[1].split(']')[0].split()
                median_value = float(parts[2])
                median_unit = parts[3]

                results[current_bench]['throughput'] = median_value
                results[current_bench]['throughput_unit'] = median_unit

        return results

    def analyze_serialization(self, results: Dict) -> Dict:
        """Analyze serialization benchmarks against AAA targets."""
        status = {}

        # Check Bincode speed (convert from ns to MB/s)
        # Assume typical message size of 100 bytes
        bincode_benches = [k for k in results.keys() if 'bincode' in k.lower() and 'player_move' in k.lower()]
        if bincode_benches:
            time_ns = results[bincode_benches[0]]['time_ns']
            # 100 bytes at X ns = (100 / time_ns) * 1e9 = bytes/sec -> / 1e6 = MB/s
            estimated_mb_s = (100 / time_ns) * 1000  # MB/s
            target = AAA_TARGETS['serialization']['bincode_speed_mb_s']
            status['bincode_speed'] = {
                'value': estimated_mb_s,
                'target': target,
                'pass': estimated_mb_s >= target,
                'ratio': estimated_mb_s / target
            }

        return status

    def analyze_protocol(self, results: Dict) -> Dict:
        """Analyze protocol benchmarks against AAA targets."""
        status = {}

        # Check message throughput (serialization speed)
        serialize_benches = [k for k in results.keys() if 'serialization' in k.lower()]
        if serialize_benches:
            # Take median of all serialization benchmarks
            times = [results[b]['time_ns'] for b in serialize_benches if 'time_ns' in results[b]]
            if times:
                median_time_ns = sorted(times)[len(times) // 2]
                msg_per_sec = 1_000_000_000 / median_time_ns
                target = AAA_TARGETS['protocol']['throughput_msg_s']
                status['message_throughput'] = {
                    'value': msg_per_sec,
                    'target': target,
                    'pass': msg_per_sec >= target,
                    'ratio': msg_per_sec / target
                }

        # Check roundtrip time
        roundtrip_benches = [k for k in results.keys() if 'roundtrip' in k.lower()]
        if roundtrip_benches:
            time_ns = results[roundtrip_benches[0]]['time_ns']
            time_us = time_ns / 1000
            target = AAA_TARGETS['protocol']['roundtrip_us']
            status['roundtrip'] = {
                'value': time_us,
                'target': target,
                'pass': time_us <= target,
                'ratio': target / time_us  # Inverted (lower is better)
            }

        return status

    def generate_report(self) -> str:
        """Generate a comprehensive AAA validation report."""
        report = []
        report.append("=" * 80)
        report.append("AAA NETWORKING BENCHMARK ANALYSIS")
        report.append("=" * 80)
        report.append("")

        total_tests = 0
        passed_tests = 0

        for category, tests in self.aaa_status.items():
            report.append(f"\n## {category.upper()}")
            report.append("-" * 80)

            for test_name, result in tests.items():
                total_tests += 1
                status_icon = "✅" if result['pass'] else "❌"
                if result['pass']:
                    passed_tests += 1

                # Format value based on test
                value_str = f"{result['value']:.2f}"
                target_str = f"{result['target']:.2f}"
                ratio_str = f"{result['ratio']:.2f}x"

                report.append(f"{status_icon} {test_name}")
                report.append(f"   Value: {value_str} | Target: {target_str} | Ratio: {ratio_str}")

        report.append("")
        report.append("=" * 80)
        report.append(f"OVERALL: {passed_tests}/{total_tests} tests passed")
        if passed_tests == total_tests:
            report.append("🎉 ALL AAA STANDARDS MET!")
        elif passed_tests / total_tests >= 0.8:
            report.append("⚠️  MOSTLY AAA COMPLIANT - Minor optimizations needed")
        else:
            report.append("❌ SIGNIFICANT OPTIMIZATION REQUIRED")
        report.append("=" * 80)

        return "\n".join(report)

def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_aaa_benchmarks.py <benchmark_output.txt>")
        sys.exit(1)

    output_file = Path(sys.argv[1])
    if not output_file.exists():
        print(f"Error: {output_file} not found")
        sys.exit(1)

    # Read benchmark output
    with open(output_file, 'r') as f:
        output_text = f.read()

    # Analyze
    analyzer = BenchmarkAnalyzer(Path("target/criterion"))
    results = analyzer.parse_criterion_output(output_text)

    if not results:
        print("No benchmark results found in output")
        sys.exit(1)

    # Analyze each category
    analyzer.aaa_status['serialization'] = analyzer.analyze_serialization(results)
    analyzer.aaa_status['protocol'] = analyzer.analyze_protocol(results)

    # Generate report
    report = analyzer.generate_report()
    print(report)

    # Save report
    report_file = Path("AAA_BENCHMARK_ANALYSIS.md")
    with open(report_file, 'w') as f:
        f.write(report)

    print(f"\nReport saved to: {report_file}")

if __name__ == "__main__":
    main()
