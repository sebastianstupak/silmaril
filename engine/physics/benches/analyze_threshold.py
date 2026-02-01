#!/usr/bin/env python3
"""
Analyze parallel threshold benchmark results to find the optimal threshold.

This script processes Criterion benchmark results and determines:
1. The crossover point where parallel processing becomes faster than sequential
2. The optimal threshold value that maximizes performance in the 1K-10K entity range
3. Performance improvement percentages for each threshold configuration

Usage:
    python analyze_threshold.py [criterion_output_dir]
"""

import json
import os
import sys
from pathlib import Path
from typing import Dict, List, Tuple


def load_benchmark_data(criterion_dir: Path) -> Dict[str, any]:
    """Load Criterion benchmark results from JSON files."""
    results = {}

    # Find all benchmark.json files
    for bench_file in criterion_dir.rglob("benchmark.json"):
        bench_name = bench_file.parent.parent.name
        group_name = bench_file.parent.name

        with open(bench_file, 'r') as f:
            data = json.load(f)

        key = f"{bench_name}/{group_name}"
        results[key] = data

    return results


def parse_estimates(data: Dict) -> Tuple[float, float]:
    """Extract mean and std deviation from Criterion estimates."""
    mean = data.get('mean', {}).get('point_estimate', 0)
    std = data.get('std_dev', {}).get('point_estimate', 0)
    return mean, std


def find_crossover_point(results: Dict[str, any]) -> int:
    """Find the entity count where parallel becomes faster than sequential."""
    crossover_candidates = []

    # Look for crossover_point benchmark group
    for key, data in results.items():
        if 'crossover_point' in key:
            # Parse entity count from key
            parts = key.split('/')
            if len(parts) >= 2:
                try:
                    entity_count = int(parts[1].split('_')[-1])
                    mean, _ = parse_estimates(data)

                    if 'sequential' in parts[1]:
                        crossover_candidates.append(('sequential', entity_count, mean))
                    elif 'parallel' in parts[1]:
                        crossover_candidates.append(('parallel', entity_count, mean))
                except (ValueError, IndexError):
                    continue

    # Group by entity count and compare sequential vs parallel
    entity_counts = set(count for _, count, _ in crossover_candidates)

    crossover = None
    for count in sorted(entity_counts):
        seq_time = next((t for mode, c, t in crossover_candidates
                        if mode == 'sequential' and c == count), None)
        par_time = next((t for mode, c, t in crossover_candidates
                        if mode == 'parallel' and c == count), None)

        if seq_time and par_time:
            if par_time < seq_time and crossover is None:
                crossover = count
                print(f"✓ Crossover point found at {count} entities")
                print(f"  Sequential: {seq_time/1e6:.3f}ms")
                print(f"  Parallel:   {par_time/1e6:.3f}ms")
                print(f"  Speedup:    {seq_time/par_time:.2f}x")
                break

    return crossover if crossover else 5000


def analyze_threshold_performance(results: Dict[str, any],
                                  target_counts: List[int]) -> Dict[int, Dict]:
    """Analyze performance for each threshold configuration."""
    threshold_performance = {}

    for key, data in results.items():
        if 'threshold_comparison' not in key and 'optimal_threshold_candidates' not in key:
            continue

        # Parse threshold and entity count from key
        parts = key.split('/')
        if len(parts) < 2:
            continue

        try:
            # Extract threshold from "threshold_1000"
            threshold_str = parts[1].split('_')[1] if 'threshold_' in parts[1] else None
            if not threshold_str:
                continue
            threshold = int(threshold_str)

            # Extract entity count from "entities_1000"
            entity_str = parts[2].split('_')[1] if len(parts) > 2 and 'entities_' in parts[2] else None
            if not entity_str:
                continue
            entity_count = int(entity_str)

            if entity_count not in target_counts:
                continue

            mean, std = parse_estimates(data)

            if threshold not in threshold_performance:
                threshold_performance[threshold] = {}

            threshold_performance[threshold][entity_count] = {
                'mean': mean,
                'std': std,
                'throughput': entity_count / (mean / 1e9)  # entities per second
            }
        except (ValueError, IndexError) as e:
            continue

    return threshold_performance


def calculate_improvement(baseline: float, optimized: float) -> float:
    """Calculate percentage improvement (positive = faster)."""
    return ((baseline - optimized) / baseline) * 100


def find_optimal_threshold(threshold_performance: Dict[int, Dict],
                           target_range: List[int]) -> Tuple[int, Dict]:
    """Find the threshold that performs best across the target range."""
    threshold_scores = {}

    for threshold, entity_data in threshold_performance.items():
        total_throughput = 0
        count = 0

        for entity_count in target_range:
            if entity_count in entity_data:
                total_throughput += entity_data[entity_count]['throughput']
                count += 1

        if count > 0:
            avg_throughput = total_throughput / count
            threshold_scores[threshold] = {
                'avg_throughput': avg_throughput,
                'coverage': count
            }

    # Find threshold with highest average throughput
    optimal = max(threshold_scores.items(),
                  key=lambda x: x[1]['avg_throughput'])

    return optimal[0], optimal[1]


def main():
    criterion_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("target/criterion")

    if not criterion_dir.exists():
        print(f"Error: Criterion directory not found: {criterion_dir}")
        sys.exit(1)

    print("=" * 70)
    print("Parallel Threshold Optimization Analysis")
    print("=" * 70)
    print()

    # Load benchmark results
    print("Loading benchmark results...")
    results = load_benchmark_data(criterion_dir)
    print(f"Loaded {len(results)} benchmark results")
    print()

    # Find crossover point
    print("1. Finding Crossover Point")
    print("-" * 70)
    crossover = find_crossover_point(results)
    print()

    # Analyze threshold performance
    print("2. Analyzing Threshold Performance")
    print("-" * 70)
    target_counts = [1_000, 2_000, 3_000, 5_000, 7_500, 10_000]
    threshold_perf = analyze_threshold_performance(results, target_counts)

    if not threshold_perf:
        print("No threshold performance data found. Run benchmarks first.")
        return

    # Find optimal threshold
    print("3. Finding Optimal Threshold")
    print("-" * 70)
    target_range = [1_000, 2_000, 3_000, 5_000, 7_500, 10_000]
    optimal_threshold, score = find_optimal_threshold(threshold_perf, target_range)

    print(f"✓ Optimal threshold: {optimal_threshold}")
    print(f"  Average throughput: {score['avg_throughput']:.0f} entities/sec")
    print(f"  Coverage: {score['coverage']}/{len(target_range)} entity counts")
    print()

    # Print detailed comparison
    print("4. Detailed Performance Comparison")
    print("-" * 70)
    print(f"{'Entity Count':<15} {'Threshold':<12} {'Time (ms)':<12} {'Throughput':<15}")
    print("-" * 70)

    for entity_count in sorted(target_range):
        print(f"\n{entity_count} entities:")
        for threshold in sorted(threshold_perf.keys()):
            if entity_count in threshold_perf[threshold]:
                data = threshold_perf[threshold][entity_count]
                time_ms = data['mean'] / 1e6
                throughput = data['throughput']
                marker = "✓" if threshold == optimal_threshold else " "
                print(f"{marker} {'':<13} {threshold:<12} {time_ms:<12.3f} {throughput:<15.0f}")

    print()
    print("=" * 70)
    print(f"RECOMMENDATION: Set PARALLEL_THRESHOLD = {optimal_threshold}")
    print("=" * 70)


if __name__ == "__main__":
    main()
