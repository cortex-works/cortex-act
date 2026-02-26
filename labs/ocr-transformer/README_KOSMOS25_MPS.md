# KOSMOS-2.5 on Apple Silicon (PyTorch + MPS)

Minimal, reproducible setup to run `microsoft/kosmos-2.5` locally on macOS with PyTorch MPS (Metal). No CUDA, no FlashAttention.

## Quickstart

```bash
# 1) Setup (creates .venv and verifies MPS)
bash scripts/setup_kosmos25.sh

# 2) Activate and run the demo (downloads weights on first run)
source .venv/bin/activate
python run_kosmos25_demo.py
```

Options:
- Override image: `IMAGE=/path/to/your.jpg python run_kosmos25_demo.py`
- Force dtype: `DTYPE=float32|float16|bfloat16`
- Lower memory: `PYTORCH_MPS_HIGH_WATERMARK_RATIO=0.0 python run_kosmos25_demo.py`

## What it does
- Loads `microsoft/kosmos-2.5`
- Uses `attn_implementation="sdpa"` to avoid FlashAttention on Mac
- Prefers `float16` on M1/M2 (use `bfloat16` if you’re on M3), falls back to `float32` if needed
- Runs two prompts on a sample receipt image:
  - `<md>`: structured Markdown description
  - `<ocr>`: OCR tokens with spatial tags

## Troubleshooting
- MPS unavailable: update macOS and Xcode CLT; ensure PyTorch is recent; check `torch.backends.mps.is_available()`
- Dtype errors: set `DTYPE=float32`
- OOM/slow: reduce `max_new_tokens`, close heavy apps, set `PYTORCH_MPS_HIGH_WATERMARK_RATIO=0.0`
- Download stalls: set `HF_HUB_ENABLE_HF_TRANSFER=1` (optional) or try another network

## Notes
- First run downloads weights into `~/.cache/huggingface`
- Subsequent runs are faster
- The demo writes `KOSMOS25_RUN_REPORT.md` with throughput and settings
