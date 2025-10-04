#!/usr/bin/env python3
"""
BlitzHash Benchmark Visualization
Generates charts from bench_results.csv
"""

import pandas as pd
import matplotlib.pyplot as plt
import sys
from pathlib import Path

def load_results(csv_path='bench_results.csv'):
    """Load benchmark results from CSV"""
    if not Path(csv_path).exists():
        print(f"❌ Error: {csv_path} not found")
        print("   Run benchmarks first: cargo run --release --bin bench")
        sys.exit(1)
    
    df = pd.read_csv(csv_path)
    print(f"✅ Loaded {len(df)} benchmark results")
    return df

def plot_throughput_comparison(df, output='bench_plot.png'):
    """Create throughput comparison chart"""
    
    # get the most recent benchmark run
    latest_timestamp = df['timestamp'].max()
    latest = df[df['timestamp'] == latest_timestamp]
    
    # group by algorithm and get mean throughput
    summary = latest.groupby('algorithm')['mb_s'].mean().sort_values()
    
    # create figure
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    fig.suptitle('BlitzHash Performance Benchmark', fontsize=16, fontweight='bold')
    
    # bar chart -absolute throughput
    colors = ['#e74c3c', '#3498db', '#2ecc71']
    bars = ax1.barh(summary.index, summary.values, color=colors[:len(summary)])
    ax1.set_xlabel('Throughput (MB/s)', fontsize=12)
    ax1.set_title('Absolute Throughput', fontsize=14)
    ax1.grid(axis='x', alpha=0.3, linestyle='--')
    
    # Add value labels on bars
    for bar in bars:
        width = bar.get_width()
        ax1.text(width, bar.get_y() + bar.get_height()/2, 
                f'{width:.1f}',
                ha='left', va='center', fontweight='bold', fontsize=10)
    
    # SPEEEEDup chart
    baseline = summary.iloc[0]  # SHA-256 is first (sorted)
    speedup = summary / baseline
    
    bars2 = ax2.barh(speedup.index, speedup.values, color=colors[:len(speedup)])
    ax2.set_xlabel('Speedup vs SHA-256', fontsize=12)
    ax2.set_title('Relative Performance', fontsize=14)
    ax2.axvline(x=1, color='red', linestyle='--', linewidth=2, alpha=0.7, label='SHA-256 baseline')
    ax2.grid(axis='x', alpha=0.3, linestyle='--')
    ax2.legend()
    
    # Add speedup labels
    for bar in bars2:
        width = bar.get_width()
        ax2.text(width, bar.get_y() + bar.get_height()/2, 
                f'{width:.2f}x',
                ha='left', va='center', fontweight='bold', fontsize=10)
    
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    print(f"📊 Chart saved: {output}")
    
    # print summary table
    print("\n" + "="*60)
    print("BENCHMARK SUMMARY")
    print("="*60)
    print(f"{'Algorithm':<20} {'MB/s':>12} {'Speedup':>10}")
    print("-"*60)
    for algo in summary.index:
        mb_s = summary[algo]
        speedup_val = mb_s / baseline
        print(f"{algo:<20} {mb_s:>12.2f} {speedup_val:>9.2f}x")
    print("="*60)

def plot_threading_scaling(df, output='threading_plot.png'):
    """Plot how performance scales with thread count"""
    
    # filter for BlitzHash parallel runs
    parallel = df[df['algorithm'] == 'BlitzHash-MT'].copy()
    
    if len(parallel) == 0:
        print("No parallel benchmark data found, skipping threading chart")
        return
    
    # group by thread count
    by_threads = parallel.groupby('threads')['mb_s'].mean().sort_index()
    
    if len(by_threads) < 2:
        print("Need multiple thread counts for scaling chart")
        return
    
    fig, ax = plt.subplots(figsize=(10, 6))
    
    ax.plot(by_threads.index, by_threads.values, 'o-', 
            linewidth=3, markersize=10, color='#2ecc71', label='BlitzHash Parallel')
    
    # ideal linear scaling line
    if len(by_threads) > 0:
        single_thread_speed = by_threads.iloc[0]
        ideal = [single_thread_speed * t for t in by_threads.index]
        ax.plot(by_threads.index, ideal, '--', 
                linewidth=2, color='#95a5a6', alpha=0.7, label='Ideal Linear Scaling')
    
    ax.set_xlabel('Thread Count', fontsize=12)
    ax.set_ylabel('Throughput (MB/s)', fontsize=12)
    ax.set_title('BlitzHash Multi-Threading Scaling', fontsize=14, fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.legend()
    
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    print(f"📊 Threading chart saved: {output}")

def plot_history(df, output='history_plot.png'):
    """Plot performance over time (multiple runs)"""
    
    if df['timestamp'].nunique() < 2:
        print("⚠️  Need multiple benchmark runs for history chart")
        return
    
    fig, ax = plt.subplots(figsize=(12, 6))
    
    for algo in df['algorithm'].unique():
        algo_data = df[df['algorithm'] == algo].sort_values('timestamp')
        timestamps = pd.to_datetime(algo_data['timestamp'], unit='s')
        ax.plot(timestamps, algo_data['mb_s'], 'o-', label=algo, linewidth=2, markersize=8)
    
    ax.set_xlabel('Benchmark Time', fontsize=12)
    ax.set_ylabel('Throughput (MB/s)', fontsize=12)
    ax.set_title('Performance History', fontsize=14, fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.legend()
    plt.xticks(rotation=45)
    
    plt.tight_layout()
    plt.savefig(output, dpi=300, bbox_inches='tight')
    print(f"📊 History chart saved: {output}")

def main():
    print("\n🎨 BlitzHash Benchmark Visualization\n")
    
    # Load data
    df = load_results()
    
    # Generate plots
    plot_throughput_comparison(df, 'bench_plot.png')
    plot_threading_scaling(df, 'threading_plot.png')
    plot_history(df, 'history_plot.png')
    
    print("\n✅ All charts generated successfully!\n")

if __name__ == '__main__':
    main()