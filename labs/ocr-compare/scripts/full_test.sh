#!/usr/bin/env bash
set -euo pipefail

IMG="${1:-1.jpg}"
GT="${2:-data/ground_truth.txt}"

echo "Running full benchmark on $IMG ..."
python scripts/run_benchmark.py --image "$IMG" --ground-truth "$GT" --engines tesseract easyocr thai_trocr paddleocr

echo
echo "Summary (reports/summary.md):"
cat reports/summary.md || true
