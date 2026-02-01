#!/usr/bin/env python3
"""
Benchmark Regression Checker

Compares benchmark results between baseline and current runs to detect performance regressions.

Supports:
- Criterion benchmark output (JSON format)
- Iai-callgrind benchmark output (instruction counts)

Usage:
    python check_benchmark_regression.py --baseline <path> --current <path> --threshold <percent> --format <criterion|iai>
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Optional


class BenchmarkResult:
    """Represents a single benchmark result."""

    def __init__(self, name: str, value: float, unit: str):
        self.name = name
        self.value = value
        self.unit = unit

    def __repr__(self):
        return f"BenchmarkResult(name={self.name}, value={self.value}, unit={self.unit})"


class Regression:
    """Represents a detected regression."""

    def __init__(self, name: str, baseline: float, current: float, change_percent: float, unit: str):
        self.name = name
        self.baseline = baseline
        self.current = current
        self.change_percent = change_percent
        self.unit = unit

    def __repr__(self):
        return (f"Regression(name={self.name}, "
                f"baseline={self.baseline}{self.unit}, "
                f"current={self.current}{self.unit}, "
                f"change={self.change_percent:+.2f}%)")


def parse_criterion_results(baseline_dir: Path, current_dir: Path) -> Tuple[Dict[str, BenchmarkResult], Dict[str, BenchmarkResult]]:
    """Parse Criterion benchmark results from JSON files.

    Criterion stores results in: target/criterion/<benchmark_name>/base/estimates.json
    """
    baseline_results = {}
    current_results = {}

    if baseline_dir.exists():
        for bench_dir in baseline_dir.iterdir():
            if bench_dir.is_dir():
                estimates_file = bench_dir / "base" / "estimates.json"
                if estimates_file.exists():
                    with open(estimates_file) as f:
                        data = json.load(f)
                        # Use median estimate (most stable)
                        median = data.get("median", {})
                        point_estimate = median.get("point_estimate", 0)
                        # Criterion times are in nanoseconds
                        baseline_results[bench_dir.name] = BenchmarkResult(
                            name=bench_dir.name,
                            value=point_estimate,
                            unit="ns"
                        )

    if current_dir.exists():
        for bench_dir in current_dir.iterdir():
            if bench_dir.is_dir():
                estimates_file = bench_dir / "base" / "estimates.json"
                if estimates_file.exists():
                    with open(estimates_file) as f:
                        data = json.load(f)
                        median = data.get("median", {})
                        point_estimate = median.get("point_estimate", 0)
                        current_results[bench_dir.name] = BenchmarkResult(
                            name=bench_dir.name,
                            value=point_estimate,
                            unit="ns"
                        )

    return baseline_results, current_results


def parse_iai_results(baseline_dir: Path, current_dir: Path) -> Tuple[Dict[str, BenchmarkResult], Dict[str, BenchmarkResult]]:
    """Parse Iai-callgrind benchmark results.

    Iai stores results in JSON format with instruction counts.
    """
    baseline_results = {}
    current_results = {}

    # Iai output structure: target/iai/<benchmark_name>/results.json
    if baseline_dir.exists():
        for result_file in baseline_dir.glob("**/results.json"):
            try:
                with open(result_file) as f:
                    data = json.load(f)
                    for bench_name, bench_data in data.items():
                        # Extract instruction count
                        instructions = bench_data.get("instructions", 0)
                        baseline_results[bench_name] = BenchmarkResult(
                            name=bench_name,
                            value=instructions,
                            unit="instructions"
                        )
            except (json.JSONDecodeError, IOError) as e:
                print(f"Warning: Failed to parse {result_file}: {e}", file=sys.stderr)

    if current_dir.exists():
        for result_file in current_dir.glob("**/results.json"):
            try:
                with open(result_file) as f:
                    data = json.load(f)
                    for bench_name, bench_data in data.items():
                        instructions = bench_data.get("instructions", 0)
                        current_results[bench_name] = BenchmarkResult(
                            name=bench_name,
                            value=instructions,
                            unit="instructions"
                        )
            except (json.JSONDecodeError, IOError) as e:
                print(f"Warning: Failed to parse {result_file}: {e}", file=sys.stderr)

    return baseline_results, current_results


def detect_regressions(
    baseline: Dict[str, BenchmarkResult],
    current: Dict[str, BenchmarkResult],
    threshold_percent: float
) -> List[Regression]:
    """Detect regressions exceeding the threshold.

    Args:
        baseline: Baseline benchmark results
        current: Current benchmark results
        threshold_percent: Regression threshold (e.g., 10.0 for 10%)

    Returns:
        List of detected regressions
    """
    regressions = []

    for name, current_result in current.items():
        if name not in baseline:
            print(f"[INFO] New benchmark: {name} ({current_result.value} {current_result.unit})")
            continue

        baseline_result = baseline[name]

        # Avoid division by zero
        if baseline_result.value == 0:
            continue

        # Calculate percentage change
        change_percent = ((current_result.value - baseline_result.value) / baseline_result.value) * 100

        # Check if it exceeds threshold (positive = regression)
        if change_percent > threshold_percent:
            regressions.append(Regression(
                name=name,
                baseline=baseline_result.value,
                current=current_result.value,
                change_percent=change_percent,
                unit=current_result.unit
            ))

    # Check for removed benchmarks
    for name in baseline:
        if name not in current:
            print(f"[WARNING] Benchmark removed: {name}", file=sys.stderr)

    return regressions


def format_value(value: float, unit: str) -> str:
    """Format a benchmark value for display."""
    if unit == "ns":
        if value > 1_000_000_000:
            return f"{value / 1_000_000_000:.2f}s"
        elif value > 1_000_000:
            return f"{value / 1_000_000:.2f}ms"
        elif value > 1_000:
            return f"{value / 1_000:.2f}µs"
        else:
            return f"{value:.2f}ns"
    elif unit == "instructions":
        if value > 1_000_000_000:
            return f"{value / 1_000_000_000:.2f}B"
        elif value > 1_000_000:
            return f"{value / 1_000_000:.2f}M"
        elif value > 1_000:
            return f"{value / 1_000:.2f}K"
        else:
            return f"{int(value)}"
    else:
        return f"{value:.2f} {unit}"


def print_report(regressions: List[Regression], threshold_percent: float):
    """Print a formatted regression report."""
    if not regressions:
        print("[OK] No regressions detected!")
        return

    print(f"[ERROR] Detected {len(regressions)} regression(s) exceeding {threshold_percent}% threshold:\n")

    # Sort by change percentage (worst first)
    regressions.sort(key=lambda r: r.change_percent, reverse=True)

    # Print table header
    print(f"{'Benchmark':<50} {'Baseline':>15} {'Current':>15} {'Change':>10}")
    print("-" * 95)

    for reg in regressions:
        baseline_str = format_value(reg.baseline, reg.unit)
        current_str = format_value(reg.current, reg.unit)
        change_str = f"{reg.change_percent:+.2f}%"

        print(f"{reg.name:<50} {baseline_str:>15} {current_str:>15} {change_str:>10}")

    print()


def main():
    parser = argparse.ArgumentParser(
        description="Check for benchmark regressions",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Check Criterion benchmarks with 20% threshold
  python check_benchmark_regression.py --baseline target/criterion-baseline --current target/criterion --threshold 20 --format criterion

  # Check Iai benchmarks with 10% threshold
  python check_benchmark_regression.py --baseline /tmp/iai-baseline/iai --current target/iai --threshold 10 --format iai
        """
    )

    parser.add_argument(
        "--baseline",
        type=Path,
        required=True,
        help="Path to baseline benchmark results directory"
    )
    parser.add_argument(
        "--current",
        type=Path,
        required=True,
        help="Path to current benchmark results directory"
    )
    parser.add_argument(
        "--threshold",
        type=float,
        required=True,
        help="Regression threshold percentage (e.g., 10 for 10%%)"
    )
    parser.add_argument(
        "--format",
        choices=["criterion", "iai"],
        required=True,
        help="Benchmark format"
    )
    parser.add_argument(
        "--fail-on-regression",
        action="store_true",
        default=True,
        help="Exit with error code if regressions detected (default: True)"
    )

    args = parser.parse_args()

    # Parse results based on format
    if args.format == "criterion":
        baseline_results, current_results = parse_criterion_results(args.baseline, args.current)
    elif args.format == "iai":
        baseline_results, current_results = parse_iai_results(args.baseline, args.current)
    else:
        print(f"Error: Unknown format {args.format}", file=sys.stderr)
        return 1

    if not baseline_results:
        print(f"[WARNING] No baseline results found in {args.baseline}", file=sys.stderr)
        print("[INFO] Skipping regression check (first run?)")
        return 0

    if not current_results:
        print(f"[ERROR] No current results found in {args.current}", file=sys.stderr)
        return 1

    print(f"[STATS] Comparing {len(baseline_results)} baseline benchmarks with {len(current_results)} current benchmarks")
    print(f"   Threshold: {args.threshold}%")
    print(f"   Format: {args.format}")
    print()

    # Detect regressions
    regressions = detect_regressions(baseline_results, current_results, args.threshold)

    # Print report
    print_report(regressions, args.threshold)

    # Exit with error if regressions found and flag is set
    if regressions and args.fail_on_regression:
        print(f"[ERROR] CI failed due to {len(regressions)} regression(s)", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
