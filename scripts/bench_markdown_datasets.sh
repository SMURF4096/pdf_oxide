#!/bin/bash
# Benchmark markdown conversion on all 6 slow PDF datasets (220 PDFs)
# Usage: ./scripts/bench_markdown_datasets.sh [binary_path] [timeout_seconds]
# Default binary: ./target/release/examples/extract_markdown_simple
# Default timeout: 60 seconds per PDF

set -euo pipefail

BINARY="${1:-./target/release/examples/extract_markdown_simple}"
TIMEOUT="${2:-60}"

BASE=~/projects/pdf_oxide_tests
DIRS=(
    "$BASE/pdfs_slow/slow_pdfs"
    "$BASE/pdfs_slow2"
    "$BASE/pdfs_slow3"
    "$BASE/pdfs_slow4"
    "$BASE/pdfs_slow5"
    "$BASE/pdfs_slow6"
)

total=0
pass=0
fail=0
slow_count=0
total_ms=0
max_ms=0
max_file=""
declare -a timings=()
declare -a failures=()
declare -a slow_files=()

for dir in "${DIRS[@]}"; do
    dataset=$(basename "$dir")
    if [ ! -d "$dir" ]; then
        echo "SKIP: $dir (not found)" >&2
        continue
    fi

    echo ""
    echo "=== Dataset: $dataset ==="
    dir_pass=0
    dir_fail=0
    dir_slow=0
    dir_total_ms=0

    for pdf in "$dir"/*.pdf; do
        [ -f "$pdf" ] || continue
        fname=$(basename "$pdf")
        total=$((total + 1))

        # Run with timeout, capture time
        start_ns=$(date +%s%N)
        if timeout "$TIMEOUT" "$BINARY" "$pdf" > /dev/null 2>/tmp/bench_md_err.txt; then
            end_ns=$(date +%s%N)
            elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
            status="pass"
            pass=$((pass + 1))
            dir_pass=$((dir_pass + 1))
            total_ms=$((total_ms + elapsed_ms))
            dir_total_ms=$((dir_total_ms + elapsed_ms))
            timings+=("$elapsed_ms")

            if [ "$elapsed_ms" -gt "$max_ms" ]; then
                max_ms=$elapsed_ms
                max_file="$dataset/$fname"
            fi

            if [ "$elapsed_ms" -gt 2000 ]; then
                slow_count=$((slow_count + 1))
                dir_slow=$((dir_slow + 1))
                slow_files+=("${elapsed_ms}ms  $dataset/$fname")
                echo "  SLOW  ${elapsed_ms}ms  $fname"
            fi
        else
            exit_code=$?
            end_ns=$(date +%s%N)
            elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
            fail=$((fail + 1))
            dir_fail=$((dir_fail + 1))

            if [ "$exit_code" -eq 124 ]; then
                echo "  TIMEOUT (>${TIMEOUT}s)  $fname"
                failures+=("TIMEOUT  $dataset/$fname")
            else
                err_msg=$(head -1 /tmp/bench_md_err.txt 2>/dev/null || echo "unknown")
                echo "  FAIL  ${elapsed_ms}ms  $fname  ($err_msg)"
                failures+=("FAIL ${elapsed_ms}ms  $dataset/$fname  ($err_msg)")
            fi
        fi
    done

    # Dataset summary
    dir_total=$((dir_pass + dir_fail))
    if [ "$dir_pass" -gt 0 ]; then
        dir_mean=$((dir_total_ms / dir_pass))
    else
        dir_mean=0
    fi
    echo "  --- $dataset: $dir_pass/$dir_total pass, $dir_slow slow (>2s), mean ${dir_mean}ms ---"
done

echo ""
echo "============================================"
echo "MARKDOWN BENCHMARK RESULTS"
echo "============================================"
echo ""
echo "Total PDFs:    $total"
echo "Pass:          $pass ($((pass * 100 / total))%)"
echo "Fail:          $fail"
echo "Slow (>2s):    $slow_count"
echo ""

if [ "$pass" -gt 0 ]; then
    mean_ms=$((total_ms / pass))
    echo "Mean time:     ${mean_ms}ms"
    echo "Max time:      ${max_ms}ms ($max_file)"

    # Compute median from sorted timings
    IFS=$'\n' sorted=($(printf '%s\n' "${timings[@]}" | sort -n)); unset IFS
    mid=$(( ${#sorted[@]} / 2 ))
    echo "Median time:   ${sorted[$mid]}ms"

    # P95
    p95_idx=$(( ${#sorted[@]} * 95 / 100 ))
    echo "P95 time:      ${sorted[$p95_idx]}ms"
fi

if [ "${#failures[@]}" -gt 0 ]; then
    echo ""
    echo "--- Failures ---"
    for f in "${failures[@]}"; do
        echo "  $f"
    done
fi

if [ "${#slow_files[@]}" -gt 0 ]; then
    echo ""
    echo "--- Slow files (>2s) ---"
    # Sort by time descending
    IFS=$'\n' sorted_slow=($(printf '%s\n' "${slow_files[@]}" | sort -t'm' -k1 -rn)); unset IFS
    for s in "${sorted_slow[@]}"; do
        echo "  $s"
    done
fi

echo ""
echo "============================================"
