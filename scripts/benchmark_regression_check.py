#!/usr/bin/env python3
"""
Benchmark Regression Checker (Enhanced)

Compares benchmark results between baseline and current runs to detect performance regressions.
Supports Criterion and Iai-callgrind formats, with detailed reporting and CI integration.

Usage:
    python benchmark_regression_check.py --baseline <path> --current <path> --threshold <percent> --format <criterion|iai>
    python benchmark_regression_check.py --baseline benchmarks/baselines/windows_main/criterion --current target/criterion --threshold 10
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass


@dataclass
class BenchmarkResult:
    """Represents a single benchmark result."""
    name: str
    value: float
    unit: str


@dataclass
class Comparison:
    """Represents a benchmark comparison."""
    name: str
    baseline: float
    current: float
    change_percent: float
    unit: str
    is_regression: bool
    is_improvement: bool


def parse_criterion_estimate(estimate_path: Path) -> Optional[float]:
    """Parse Criterion's estimates.json file."""
    try:
        with open(estimate_path, 'r') as f:
            data = json.load(f)
            # Criterion stores median estimate in nanoseconds
            if 'median' in data and 'point_estimate' in data['median']:
                return data['median']['point_estimate']
            elif 'point_estimate' in data:
                return data['point_estimate']
    except (FileNotFoundError, json.JSONDecodeError, KeyError):
        pass
    return None


def parse_criterion_results(directory: Path) -> Dict[str, BenchmarkResult]:
    """Parse all Criterion benchmark results from directory."""
    results = {}

    if not directory.exists():
        return results

    # Criterion structure: <benchmark_name>/base/estimates.json
    for bench_dir in directory.iterdir():
        if not bench_dir.is_dir():
            continue

        estimate_file = bench_dir / "base" / "estimates.json"
        value = parse_criterion_estimate(estimate_file)

        if value is not None:
            results[bench_dir.name] = BenchmarkResult(
                name=bench_dir.name,
                value=value,
                unit="ns"
            )

        # Also check for nested benchmarks
        for sub_dir in bench_dir.iterdir():
            if sub_dir.is_dir():
                sub_estimate = sub_dir / "base" / "estimates.json"
                sub_value = parse_criterion_estimate(sub_estimate)
                if sub_value is not None:
                    full_name = f"{bench_dir.name}/{sub_dir.name}"
                    results[full_name] = BenchmarkResult(
                        name=full_name,
                        value=sub_value,
                        unit="ns"
                    )

    return results


def parse_iai_results(directory: Path) -> Dict[str, BenchmarkResult]:
    """Parse Iai-callgrind benchmark results."""
    results = {}

    if not directory.exists():
        return results

    # Iai stores results in JSON format
    for json_file in directory.rglob("*.json"):
        try:
            with open(json_file, 'r') as f:
                data = json.load(f)
                # Iai structure varies, try to extract instruction counts
                if isinstance(data, dict):
                    for bench_name, bench_data in data.items():
                        if isinstance(bench_data, dict) and 'instructions' in bench_data:
                            results[bench_name] = BenchmarkResult(
                                name=bench_name,
                                value=bench_data['instructions'],
                                unit="instructions"
                            )
        except (json.JSONDecodeError, KeyError):
            continue

    return results


def compare_results(
    baseline: Dict[str, BenchmarkResult],
    current: Dict[str, BenchmarkResult],
    threshold_percent: float
) -> List[Comparison]:
    """Compare baseline and current results, detecting regressions."""
    comparisons = []

    # Find all benchmark names (union of both sets)
    all_names = set(baseline.keys()) | set(current.keys())

    for name in sorted(all_names):
        if name not in baseline:
            # New benchmark (not a regression)
            continue

        if name not in current:
            # Removed benchmark (warn but not a regression)
            print(f"Warning: Benchmark '{name}' removed")
            continue

        baseline_val = baseline[name].value
        current_val = current[name].value
        unit = baseline[name].unit

        # Calculate percentage change
        if baseline_val == 0:
            change_percent = 0 if current_val == 0 else float('inf')
        else:
            change_percent = ((current_val - baseline_val) / baseline_val) * 100

        is_regression = change_percent > threshold_percent
        is_improvement = change_percent < -threshold_percent

        comparisons.append(Comparison(
            name=name,
            baseline=baseline_val,
            current=current_val,
            change_percent=change_percent,
            unit=unit,
            is_regression=is_regression,
            is_improvement=is_improvement
        ))

    return comparisons


def format_value(value: float, unit: str) -> str:
    """Format benchmark value with appropriate unit and precision."""
    if unit == "ns":
        if value < 1000:
            return f"{value:.1f} ns"
        elif value < 1_000_000:
            return f"{value/1000:.1f} μs"
        elif value < 1_000_000_000:
            return f"{value/1_000_000:.1f} ms"
        else:
            return f"{value/1_000_000_000:.1f} s"
    elif unit == "instructions":
        if value < 1000:
            return f"{value:.0f} instr"
        elif value < 1_000_000:
            return f"{value/1000:.1f}K instr"
        else:
            return f"{value/1_000_000:.1f}M instr"
    else:
        return f"{value:.2f} {unit}"


def print_comparison_table(comparisons: List[Comparison], show_all: bool = False):
    """Print comparison results as formatted table."""
    if not comparisons:
        print("No benchmarks to compare.")
        return

    # Filter results if not showing all
    if not show_all:
        display_comparisons = [c for c in comparisons if c.is_regression or c.is_improvement]
    else:
        display_comparisons = comparisons

    if not display_comparisons:
        print("[OK] No significant changes detected.")
        return

    # Header
    print("\n" + "="*80)
    print("BENCHMARK COMPARISON RESULTS")
    print("="*80)
    print(f"{'Benchmark':<40} {'Baseline':<15} {'Current':<15} {'Change':>10}")
    print("-"*80)

    # Results
    for comp in sorted(display_comparisons, key=lambda x: abs(x.change_percent), reverse=True):
        marker = ""
        if comp.is_regression:
            marker = "[ERROR]"
        elif comp.is_improvement:
            marker = "[OK]   "
        else:
            marker = "       "

        name = comp.name[:38] if len(comp.name) > 38 else comp.name

        print(f"{marker} {name:<38} "
              f"{format_value(comp.baseline, comp.unit):<15} "
              f"{format_value(comp.current, comp.unit):<15} "
              f"{comp.change_percent:+9.2f}%")

    print("="*80)


def generate_markdown_report(
    comparisons: List[Comparison],
    threshold: float,
    output_file: Path
):
    """Generate detailed markdown report."""
    regressions = [c for c in comparisons if c.is_regression]
    improvements = [c for c in comparisons if c.is_improvement]
    unchanged = [c for c in comparisons if not c.is_regression and not c.is_improvement]

    report = f"""# Benchmark Regression Report

**Threshold:** ±{threshold}%
**Total Benchmarks:** {len(comparisons)}
**Regressions:** {len(regressions)}
**Improvements:** {len(improvements)}
**Unchanged:** {len(unchanged)}

---

"""

    if regressions:
        report += "## Regressions Detected\n\n"
        report += "| Benchmark | Baseline | Current | Change |\n"
        report += "|-----------|----------|---------|--------|\n"

        for comp in sorted(regressions, key=lambda x: x.change_percent, reverse=True):
            report += f"| {comp.name} | {format_value(comp.baseline, comp.unit)} | "
            report += f"{format_value(comp.current, comp.unit)} | "
            report += f"{comp.change_percent:+.2f}% |\n"

        report += "\n"

    if improvements:
        report += "## Improvements Detected\n\n"
        report += "| Benchmark | Baseline | Current | Change |\n"
        report += "|-----------|----------|---------|--------|\n"

        for comp in sorted(improvements, key=lambda x: x.change_percent):
            report += f"| {comp.name} | {format_value(comp.baseline, comp.unit)} | "
            report += f"{format_value(comp.current, comp.unit)} | "
            report += f"{comp.change_percent:+.2f}% |\n"

        report += "\n"

    if unchanged:
        report += f"## Unchanged ({len(unchanged)} benchmarks)\n\n"
        report += "<details>\n<summary>Click to expand</summary>\n\n"
        report += "| Benchmark | Baseline | Current | Change |\n"
        report += "|-----------|----------|---------|--------|\n"

        for comp in sorted(unchanged, key=lambda x: abs(x.change_percent), reverse=True):
            report += f"| {comp.name} | {format_value(comp.baseline, comp.unit)} | "
            report += f"{format_value(comp.current, comp.unit)} | "
            report += f"{comp.change_percent:+.2f}% |\n"

        report += "\n</details>\n\n"

    report += "---\n\n"
    report += "**Generated by:** `scripts/benchmark_regression_check.py`\n"

    with open(output_file, 'w') as f:
        f.write(report)

    print(f"\n[REPORT] Detailed report saved to: {output_file}")


def main():
    parser = argparse.ArgumentParser(
        description="Check for benchmark performance regressions",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Compare Criterion benchmarks with 10% threshold
  python benchmark_regression_check.py \\
    --baseline benchmarks/baselines/windows_main/criterion \\
    --current target/criterion \\
    --threshold 10 \\
    --format criterion

  # Compare Iai benchmarks (stricter threshold)
  python benchmark_regression_check.py \\
    --baseline benchmarks/baselines/linux_main/iai \\
    --current target/iai \\
    --threshold 5 \\
    --format iai \\
    --fail-on-regression
        """
    )

    parser.add_argument(
        '--baseline',
        type=Path,
        required=True,
        help='Path to baseline benchmark directory'
    )

    parser.add_argument(
        '--current',
        type=Path,
        required=True,
        help='Path to current benchmark directory'
    )

    parser.add_argument(
        '--threshold',
        type=float,
        required=True,
        help='Regression threshold percentage (e.g., 10 for 10%%)'
    )

    parser.add_argument(
        '--format',
        choices=['criterion', 'iai'],
        default='criterion',
        help='Benchmark format (default: criterion)'
    )

    parser.add_argument(
        '--output',
        type=Path,
        help='Output markdown report file (optional)'
    )

    parser.add_argument(
        '--fail-on-regression',
        action='store_true',
        help='Exit with error code if regressions detected'
    )

    parser.add_argument(
        '--show-all',
        action='store_true',
        help='Show all benchmarks, not just changes'
    )

    args = parser.parse_args()

    # Parse results based on format
    if args.format == 'criterion':
        print(f"Parsing Criterion benchmarks...")
        print(f"  Baseline: {args.baseline}")
        print(f"  Current:  {args.current}")
        baseline_results = parse_criterion_results(args.baseline)
        current_results = parse_criterion_results(args.current)
    else:  # iai
        print(f"Parsing Iai-callgrind benchmarks...")
        print(f"  Baseline: {args.baseline}")
        print(f"  Current:  {args.current}")
        baseline_results = parse_iai_results(args.baseline)
        current_results = parse_iai_results(args.current)

    if not baseline_results:
        print(f"[ERROR] No baseline results found in {args.baseline}")
        return 1

    if not current_results:
        print(f"[ERROR] No current results found in {args.current}")
        return 1

    print(f"Found {len(baseline_results)} baseline benchmarks")
    print(f"Found {len(current_results)} current benchmarks")

    # Compare results
    comparisons = compare_results(baseline_results, current_results, args.threshold)

    # Print table
    print_comparison_table(comparisons, args.show_all)

    # Generate markdown report if requested
    if args.output:
        generate_markdown_report(comparisons, args.threshold, args.output)

    # Count regressions and improvements
    regressions = [c for c in comparisons if c.is_regression]
    improvements = [c for c in comparisons if c.is_improvement]

    # Summary
    print(f"\n[SUMMARY] Summary:")
    print(f"   Total:           {len(comparisons)} benchmarks")
    print(f"   [ERROR] Regressions:  {len(regressions)}")
    print(f"   [OK] Improvements: {len(improvements)}")
    print(f"   Threshold:       ±{args.threshold}%")

    # Exit code
    if regressions:
        print(f"\n[WARNING] {len(regressions)} regression(s) detected!")
        if args.fail_on_regression:
            print("Failing due to --fail-on-regression flag")
            return 1
        else:
            print("(Use --fail-on-regression to fail CI on regressions)")
            return 0
    else:
        print("\n[OK] No regressions detected!")
        return 0


if __name__ == "__main__":
    sys.exit(main())
