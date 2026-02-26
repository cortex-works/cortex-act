OCR Benchmark (Thai + English)

This project benchmarks multiple OCR engines on a single image with a shared preprocessing pipeline and Thai-aware accuracy metrics (CER/WER).

Engines supported:
- Tesseract (tha+eng)
- EasyOCR (th+en)
- Thai‑TrOCR (Transformer fine-tuned for Thai/English)
- PaddleOCR (optional; via local install; limited Thai support in official wheels)

Quick start

1) Python env (macOS)
- Install Homebrew Tesseract (for Tesseract engine):
  - brew install tesseract tesseract-lang
- Create venv and install deps:
  - python3 -m venv .venv
  - source .venv/bin/activate
  - pip install --upgrade pip
  - pip install -r requirements.txt

2) Ground truth
- Put expected text for `1.jpg` in `ground_truth.txt` (UTF-8). If omitted, metrics will be skipped for that engine.

3) Run benchmark
- python benchmark.py --image 1.jpg --gt ground_truth.txt --engines tesseract,easyocr,trocr,paddle --save-debug

4) Results
- Outputs per engine: `outputs/`
- Reports: `reports/summary.json` and `reports/summary.md`
- Preprocessing debug images (if `--save-debug`): `debug/`

Notes on PaddleOCR
- PaddleOCR on macOS ARM can be finicky. The default requirements do NOT install it. If you want to try it, install `paddlepaddle` and `paddleocr` manually as noted in `requirements.txt` (commented lines). Thai (`lang='th'`) is not officially supported in older wheels; the code will fallback to `latin` if `th` is unavailable, so Thai accuracy may be poor. Use Thai‑TrOCR and EasyOCR for Thai text.

Engines
- Tesseract: Requires `tesseract` CLI in PATH; uses `tha+eng` and psm=6.
- EasyOCR: Reader(["th","en"]).
- Thai‑TrOCR: Uses `openthaigpt/thai-trocr` from Hugging Face; CPU-friendly but slower.
- PaddleOCR: Attempts local Python import and falls back to `latin` if `th` unsupported. You can omit it.

Preprocessing
- Upscale (min side 1200 px) with area interpolation
- Grayscale
- Light deskew (±5° search)
- Sauvola binarization

Metrics
- Normalization: Unicode NFKC + collapse whitespace
- CER: Levenshtein distance / characters
- WER: Jiwer WER with Thai tokenization (PyThaiNLP newmm) when Thai text is detected; otherwise whitespace tokenization.

Troubleshooting
- Tesseract not found: install via Homebrew and restart shell.
- Torch install slow on macOS: it’s CPU-only; be patient. You can skip `trocr` by removing it from `--engines`.
- PaddleOCR install on Apple Silicon can be unstable. If it fails, exclude `paddle` and rely on EasyOCR + Thai‑TrOCR for Thai.

License
Apache 2.0
# OCR Compare (Thai–English)

This project evaluates multiple OCR engines on the same input image (`1.jpg`) with a shared preprocessing pipeline. Metrics: CER, WER, runtime.

Tools: Tesseract, EasyOCR, PaddleOCR, Thai-TrOCR (if available on your hardware), optional MMOCR.

## Quick start

- Requirements: Python 3.10+, macOS, optionally Homebrew. GPU acceleration is optional; this setup runs on CPU.

- Create and activate a venv, install deps, and run the benchmark script. See the bottom of this file for commands.

## Ground truth

Edit `data/ground_truth.txt` with the manually verified transcription for `1.jpg`.

## Outputs

- `outputs/*.txt` raw OCR results per engine
- `reports/summary.json` metrics and timings
- `reports/summary.md` consolidated report

## Optional

- You can drop more test images into project root and reference them via CLI.

## Commands (optional to copy)

```bash
# 1) Create venv
python3 -m venv .venv
source .venv/bin/activate

# 2) Install system deps (tesseract + thai)
brew install tesseract tesseract-lang

# 3) Install Python deps
pip install -r requirements.txt

# 4) Put ground truth text
cp -n data/ground_truth.example.txt data/ground_truth.txt || true

# 5) Run benchmark on 1.jpg
python scripts/run_benchmark.py --image 1.jpg --ground-truth data/ground_truth.txt
 
# 6) Re-run only PaddleOCR via Docker (if local paddlepaddle is unavailable)
#    This can take several minutes the first time to pull image and download models.
python scripts/run_benchmark.py --image 1.jpg --ground-truth data/ground_truth.txt --engines paddleocr

## PaddleOCR via Docker

If `paddleocr` cannot import locally on macOS ARM, the benchmark will automatically fall back to a Docker container based on `paddlepaddle/paddle:2.6.1` and run a small helper script (`scripts/paddle_ocr_infer.py`). Ensure Docker Desktop is running. The first run is slow due to image/model downloads; subsequent runs are much faster.

Troubleshooting:
- If you interrupt a first-time run, remove partial images and re-run.
- To run manually and inspect logs:

```bash
# optional: run PaddleOCR alone in a container
IMG=/work/1.jpg
docker run --rm -v "$PWD":/work -w /work paddlepaddle/paddle:2.6.1 \
	bash -lc "python3 -m pip install paddleocr && python3 scripts/paddle_ocr_infer.py --image $IMG"
```

## Current results on `1.jpg` (CPU; Thai-aware WER)

| Tool      | CER   | WER   | Speed (s) |
|-----------|-------|-------|-----------|
| Tesseract | 0.173 | 4.000 | ~1.06     |
| EasyOCR   | 0.100 | 0.353 | ~4.1      |
| PaddleOCR | —     | —     | —         |
| Thai-TrOCR| 1.000 | 1.000 | ~4.6      |

Notes:
- EasyOCR produced the best Thai accuracy on this sample; Tesseract was faster but less accurate. Thai‑TrOCR (generic checkpoint) was not competitive; a Thai‑fine-tuned model is recommended if available. PaddleOCR is expected to be strong; run the Docker path above to add its numbers to the report automatically.
```
