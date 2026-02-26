# Typhoon OCR (Thai/English) on Apple Silicon (MPS)

This adds a fast local runner for `scb10x/typhoon-ocr-3b` (Qwen2.5-VL based) using PyTorch MPS on macOS. It takes an image and outputs a single JSON with the key `natural_text` containing Thai/English OCR in markdown.

## Quick start

1) Setup (creates `.venv` and installs deps):

```bash
bash scripts/setup_typhoon_ocr.sh
```

2) Activate and run on an image (defaults to `1.jpg`):

```bash
source .venv/bin/activate
# optional knobs
export DEVICE=mps            # or cpu
export DTYPE=bf16            # bf16|fp16|fp32 (bf16 recommended on M3/M4)
export IMAGE_MAX_SIDE=1600   # downscale for speed
export MAX_NEW_TOKENS=4096   # generation budget
export TEMPERATURE=0.1
export VERBOSE=1

python3 run_typhoon_ocr.py 1.jpg > output.json
```

This prints a single JSON to stdout and also writes `output_ocr_parsed.json`.

## Notes
- Model: `scb10x/typhoon-ocr-3b` (Apache-2.0). Base: Qwen2.5-VL 3B.
- Prompting: The script uses the recommended "default" prompt and asks the model to return JSON with `natural_text`.
- Performance: On M3/M4, prefer `DTYPE=bf16` and keep `IMAGE_MAX_SIDE` ≤ 1600. Increase `MAX_NEW_TOKENS` for long pages.
- API alternative: For best performance, the authors recommend vLLM or their hosted API. You can also `pip install typhoon-ocr` and use `ocr_document()` against a local vLLM server.

## Output schema

```json
{
  "natural_text": "...markdown content in Thai/English..."
}
```

If the model returns non-JSON content, the runner wraps it into this schema.

## Next steps
- Map `natural_text` into our structured fields (ship_to, from, details, tracking) by reusing the existing parser.
- Add an option to run via vLLM server for speed and multi-page PDFs using `typhoon_ocr` utilities.
