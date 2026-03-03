#!/bin/bash
# Benchmark comparison across v0.3.11, v0.3.12, v0.3.13
# Usage: bench_compare_versions.sh <version_label> <binary_path> <output_csv>
# Runs a single version on all 6 datasets, outputs CSV with per-PDF timing

set -euo pipefail

VERSION="$1"
BINARY="$2"
OUTPUT="$3"
TIMEOUT=60

BASE=~/projects/pdf_oxide_tests
DIRS=(
    "$BASE/pdfs_slow/slow_pdfs"
    "$BASE/pdfs_slow2"
    "$BASE/pdfs_slow3"
    "$BASE/pdfs_slow4"
    "$BASE/pdfs_slow5"
    "$BASE/pdfs_slow6"
)

echo "version,dataset,filename,status,time_ms,pages,chars" > "$OUTPUT"

total=0
pass=0
fail=0
slow=0

for dir in "${DIRS[@]}"; do
    dataset=$(basename "$dir")
    if [ ! -d "$dir" ]; then
        echo "SKIP: $dir (not found)" >&2
        continue
    fi

    echo "=== $VERSION: $dataset ===" >&2
    dir_pass=0
    dir_fail=0

    for pdf in "$dir"/*.pdf; do
        [ -f "$pdf" ] || continue
        fname=$(basename "$pdf")
        total=$((total + 1))

        # Run with timeout, capture time
        start_ns=$(date +%s%N)
        output=$(timeout "$TIMEOUT" "$BINARY" "$pdf" 2>/dev/null) && status="pass" || status="fail"
        end_ns=$(date +%s%N)
        elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))

        # Count chars from output
        chars=${#output}

        if [ "$status" = "pass" ]; then
            pass=$((pass + 1))
            dir_pass=$((dir_pass + 1))
            if [ "$elapsed_ms" -gt 2000 ]; then
                slow=$((slow + 1))
                echo "  SLOW  ${elapsed_ms}ms  $fname" >&2
            fi
        else
            fail=$((fail + 1))
            dir_fail=$((dir_fail + 1))
            echo "  FAIL  ${elapsed_ms}ms  $fname" >&2
        fi

        echo "$VERSION,$dataset,\"$fname\",$status,$elapsed_ms,,$chars" >> "$OUTPUT"
    done

    echo "  --- $dataset: $dir_pass pass, $dir_fail fail ---" >&2
done

echo "" >&2
echo "=== $VERSION TOTAL: $pass pass, $fail fail, $slow slow (of $total) ===" >&2
