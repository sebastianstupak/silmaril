#!/usr/bin/env python3
"""
compare_with_industry.py - Compare benchmark results with industry standards

This script reads Criterion benchmark results and compares them against
industry baselines documented in PLATFORM_BENCHMARK_COMPARISON.md.

Usage:
    python scripts/compare_with_industry.py --results benchmarks/results/windows_20260201_120000
    python scripts/compare_with_industry.py --results benchmarks/results/linux_20260201_120000 --output report.md
"""

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple, Optional
import re

# Industry baseline targets (from PLATFORM_BENCHMARK_COMPARISON.md)
INDUSTRY_TARGETS = {
    # Platform abstraction - Time backend (nanoseconds)
    "platform/time/monotonic_nanos": {
        "target_ns": 50,
        "goal_ns": 30,
        "industry_min": 26,  # Linux clock_gettime
        "industry_max": 300,  # Windows QPC TSC
        "category": "Platform: Time Query",
        "notes": "Our target: <50ns (goal: 30ns), Industry: 26-300ns"
    },
    "platform/time/now": {
        "target_ns": 100,
        "goal_ns": 50,
        "industry_min": 40,
        "industry_max": 500,
        "category": "Platform: Time Query",
        "notes": "Includes conversion overhead"
    },

    # Platform abstraction - Threading (microseconds)
    "platform/threading/set_priority": {
        "target_us": 5,
        "goal_us": 2,
        "industry_min": 1,
        "industry_max": 5,
        "category": "Platform: Threading",
        "notes": "System call overhead"
    },
    "platform/threading/set_affinity": {
        "target_us": 10,
        "goal_us": 5,
        "industry_min": 5,
        "industry_max": 15,
        "category": "Platform: Threading",
        "notes": "CPU pinning overhead"
    },

    # Platform abstraction - Filesystem (nanoseconds)
    "platform/fs/normalize_path_simple": {
        "target_ns": 500,
        "goal_ns": 200,
        "industry_min": 100,
        "industry_max": 500,
        "category": "Platform: Filesystem",
        "notes": "Simple path normalization"
    },
    "platform/fs/normalize_path_complex": {
        "target_ns": 2000,
        "goal_ns": 1000,
        "industry_min": 1000,
        "industry_max": 3000,
        "category": "Platform: Filesystem",
        "notes": "Path with .. and ."
    },

    # ECS operations (nanoseconds)
    "ecs/entity/spawn": {
        "target_ns": 500,
        "goal_ns": 300,
        "industry_min": 5,  # EnTT: 4.9ns
        "industry_max": 1000,
        "category": "ECS: Entity",
        "notes": "Entity creation overhead"
    },
    "ecs/query/iter_1_component": {
        "target_ns": 50,
        "goal_ns": 20,
        "industry_min": 1,  # EnTT: 0.8ns
        "industry_max": 100,
        "category": "ECS: Query",
        "notes": "Per-entity iteration cost"
    },
    "ecs/query/iter_2_components": {
        "target_ns": 100,
        "goal_ns": 50,
        "industry_min": 5,
        "industry_max": 200,
        "category": "ECS: Query",
        "notes": "Two-component query iteration"
    },

    # Profiling overhead (nanoseconds)
    "profiling/scope_creation_on": {
        "target_ns": 200,
        "goal_ns": 100,
        "industry_min": 50,
        "industry_max": 500,
        "category": "Profiling",
        "notes": "Profiling enabled overhead"
    },
    "profiling/scope_creation_off": {
        "target_ns": 1,
        "goal_ns": 0,
        "industry_min": 0,
        "industry_max": 10,
        "category": "Profiling",
        "notes": "Zero-cost when disabled"
    },
}


def parse_criterion_estimate(estimate_path: Path) -> Optional[float]:
    """Parse Criterion's estimates.json file to get median time in nanoseconds."""
    try:
        with open(estimate_path, 'r') as f:
            data = json.load(f)
            # Criterion stores point estimate in nanoseconds
            if 'median' in data and 'point_estimate' in data['median']:
                return data['median']['point_estimate']
            elif 'point_estimate' in data:
                return data['point_estimate']
    except (FileNotFoundError, json.JSONDecodeError, KeyError) as e:
        print(f"Warning: Could not parse {estimate_path}: {e}")
    return None


def find_benchmarks(criterion_dir: Path) -> Dict[str, float]:
    """Find all benchmark results in Criterion output directory."""
    results = {}

    if not criterion_dir.exists():
        print(f"Error: Criterion directory not found: {criterion_dir}")
        return results

    # Criterion structure: target/criterion/<benchmark_name>/<group_name>/base/estimates.json
    for bench_dir in criterion_dir.iterdir():
        if not bench_dir.is_dir():
            continue

        # Look for estimates.json in all subdirectories
        for estimate_file in bench_dir.rglob("estimates.json"):
            # Build benchmark identifier from path
            rel_path = estimate_file.relative_to(criterion_dir)
            parts = list(rel_path.parts[:-2])  # Remove 'base' and 'estimates.json'
            bench_id = "/".join(parts)

            time_ns = parse_criterion_estimate(estimate_file)
            if time_ns is not None:
                results[bench_id] = time_ns

    return results


def categorize_benchmarks(results: Dict[str, float]) -> Dict[str, List[Tuple[str, float]]]:
    """Group benchmarks by category."""
    categories = {}

    for bench_name, time_ns in results.items():
        # Determine category from benchmark name
        category = "Other"
        if "platform" in bench_name.lower():
            if "time" in bench_name.lower():
                category = "Platform: Time Query"
            elif "thread" in bench_name.lower():
                category = "Platform: Threading"
            elif "fs" in bench_name.lower() or "file" in bench_name.lower():
                category = "Platform: Filesystem"
            else:
                category = "Platform: Other"
        elif "ecs" in bench_name.lower() or "entity" in bench_name.lower() or "query" in bench_name.lower():
            if "entity" in bench_name.lower():
                category = "ECS: Entity"
            elif "query" in bench_name.lower():
                category = "ECS: Query"
            else:
                category = "ECS: Other"
        elif "profil" in bench_name.lower():
            category = "Profiling"
        elif "physic" in bench_name.lower():
            category = "Physics"
        elif "math" in bench_name.lower() or "simd" in bench_name.lower():
            category = "Math/SIMD"
        elif "serial" in bench_name.lower():
            category = "Serialization"

        if category not in categories:
            categories[category] = []
        categories[category].append((bench_name, time_ns))

    return categories


def format_time(ns: float) -> str:
    """Format time with appropriate unit."""
    if ns < 1000:
        return f"{ns:.1f} ns"
    elif ns < 1_000_000:
        return f"{ns/1000:.1f} μs"
    elif ns < 1_000_000_000:
        return f"{ns/1_000_000:.1f} ms"
    else:
        return f"{ns/1_000_000_000:.1f} s"


def assess_performance(bench_name: str, actual_ns: float) -> Tuple[str, str, str]:
    """
    Assess performance against targets.
    Returns: (status, marker, explanation)
    """
    # Try to find matching target
    target_info = None
    for target_name, info in INDUSTRY_TARGETS.items():
        if target_name in bench_name:
            target_info = info
            break

    if not target_info:
        return "unknown", "[UNKNOWN]", "No industry baseline available"

    # Convert to appropriate unit
    if "target_us" in target_info:
        target = target_info["target_us"] * 1000
        goal = target_info["goal_us"] * 1000
    else:
        target = target_info["target_ns"]
        goal = target_info["goal_ns"]

    industry_min = target_info.get("industry_min", 0)
    industry_max = target_info.get("industry_max", float('inf'))

    # Assess
    if actual_ns <= goal:
        return "excellent", "[EXCELLENT]", f"Meets goal ({format_time(goal)})"
    elif actual_ns <= target:
        return "good", "[GOOD]", f"Within target ({format_time(target)})"
    elif actual_ns <= industry_max:
        return "acceptable", "[ACCEPTABLE]", f"Competitive with industry max ({format_time(industry_max)})"
    else:
        return "poor", "[POOR]", f"Exceeds industry max ({format_time(industry_max)})"


def generate_markdown_report(results: Dict[str, float], output_file: Path, platform: str):
    """Generate detailed markdown comparison report."""
    categories = categorize_benchmarks(results)

    report = f"""# Benchmark Industry Comparison Report

**Platform:** {platform}
**Date:** {Path(output_file).parent.name}
**Total Benchmarks:** {len(results)}

---

## Executive Summary

This report compares our benchmark results against industry standards documented in
`PLATFORM_BENCHMARK_COMPARISON.md`.

### Performance Assessment Legend

- [EXCELLENT] **Excellent**: Meets or exceeds goal performance
- [GOOD] **Good**: Within target performance
- [ACCEPTABLE] **Acceptable**: Competitive with industry maximum
- [POOR] **Poor**: Exceeds industry maximum (needs optimization)
- [UNKNOWN] **Unknown**: No industry baseline available

---

"""

    # Summary statistics
    status_counts = {"excellent": 0, "good": 0, "acceptable": 0, "poor": 0, "unknown": 0}

    for category, benchmarks in sorted(categories.items()):
        report += f"## {category}\n\n"
        report += "| Benchmark | Actual | Status | Assessment |\n"
        report += "|-----------|--------|--------|------------|\n"

        for bench_name, actual_ns in sorted(benchmarks):
            status, marker, explanation = assess_performance(bench_name, actual_ns)
            status_counts[status] += 1

            report += f"| {bench_name} | {format_time(actual_ns)} | {marker} | {explanation} |\n"

        report += "\n"

    # Add summary at the end
    report += "---\n\n## Summary Statistics\n\n"
    total = sum(status_counts.values())
    report += f"- [EXCELLENT] Excellent: {status_counts['excellent']} ({100*status_counts['excellent']/total:.1f}%)\n"
    report += f"- [GOOD] Good: {status_counts['good']} ({100*status_counts['good']/total:.1f}%)\n"
    report += f"- [ACCEPTABLE] Acceptable: {status_counts['acceptable']} ({100*status_counts['acceptable']/total:.1f}%)\n"
    report += f"- [POOR] Poor: {status_counts['poor']} ({100*status_counts['poor']/total:.1f}%)\n"
    report += f"- [UNKNOWN] Unknown: {status_counts['unknown']} ({100*status_counts['unknown']/total:.1f}%)\n"
    report += "\n"

    # Recommendations
    report += "---\n\n## Recommendations\n\n"

    poor_benchmarks = [(name, time) for category in categories.values()
                       for name, time in category
                       if assess_performance(name, time)[0] == "poor"]

    if poor_benchmarks:
        report += "### High Priority (Performance Issues)\n\n"
        for name, time_ns in poor_benchmarks:
            report += f"- **{name}**: {format_time(time_ns)} - Investigate and optimize\n"
        report += "\n"

    acceptable_benchmarks = [(name, time) for category in categories.values()
                             for name, time in category
                             if assess_performance(name, time)[0] == "acceptable"]

    if acceptable_benchmarks:
        report += "### Medium Priority (Optimization Opportunities)\n\n"
        for name, time_ns in acceptable_benchmarks[:5]:  # Show top 5
            report += f"- **{name}**: {format_time(time_ns)} - Could be improved\n"
        report += "\n"

    if not poor_benchmarks and not acceptable_benchmarks:
        report += "All benchmarks meet or exceed industry standards!\n\n"

    report += "---\n\n"
    report += "**Generated by:** `scripts/compare_with_industry.py`\n"
    report += "**Source:** `PLATFORM_BENCHMARK_COMPARISON.md`\n"

    # Write report
    with open(output_file, 'w') as f:
        f.write(report)

    print(f"Report generated: {output_file}")

    # Print summary to console
    print("\n" + "="*60)
    print("BENCHMARK SUMMARY")
    print("="*60)
    print(f"[EXCELLENT] Excellent: {status_counts['excellent']}")
    print(f"[GOOD] Good:           {status_counts['good']}")
    print(f"[ACCEPTABLE] Acceptable: {status_counts['acceptable']}")
    print(f"[POOR] Poor:           {status_counts['poor']}")
    print(f"[UNKNOWN] Unknown:     {status_counts['unknown']}")
    print("="*60)

    if poor_benchmarks:
        print("\n[WARNING] Some benchmarks exceed industry standards!")
        return 1

    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Compare benchmark results with industry standards",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python scripts/compare_with_industry.py --results benchmarks/results/windows_20260201_120000
  python scripts/compare_with_industry.py --results benchmarks/results/linux_20260201_120000 --output custom_report.md
        """
    )

    parser.add_argument(
        '--results',
        type=Path,
        required=True,
        help='Path to results directory from benchmark_all_platforms.sh'
    )

    parser.add_argument(
        '--output',
        type=Path,
        help='Output markdown file (default: <results_dir>/industry_comparison.md)'
    )

    parser.add_argument(
        '--criterion-dir',
        type=Path,
        default=Path('target/criterion'),
        help='Path to Criterion output directory (default: target/criterion)'
    )

    args = parser.parse_args()

    # Determine platform from results directory name
    platform = "unknown"
    if "windows" in args.results.name:
        platform = "Windows"
    elif "linux" in args.results.name:
        platform = "Linux"
    elif "macos" in args.results.name:
        platform = "macOS"

    # Find benchmarks
    print(f"Searching for benchmarks in: {args.criterion_dir}")
    results = find_benchmarks(args.criterion_dir)

    if not results:
        print("Error: No benchmark results found!")
        print(f"Make sure benchmarks have been run and Criterion output exists at: {args.criterion_dir}")
        return 1

    print(f"Found {len(results)} benchmark results")

    # Generate report
    output_file = args.output or (args.results / "industry_comparison.md")
    return generate_markdown_report(results, output_file, platform)


if __name__ == "__main__":
    sys.exit(main())
