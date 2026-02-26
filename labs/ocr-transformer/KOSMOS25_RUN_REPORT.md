# KOSMOS-2.5 MPS Run Report

Device: mps
DType: torch.float32
PyTorch: 2.8.0
Transformers: auto
Image: shipping_label_02.png

Throughput (approx):
- <md>:  1.96 tok/s
- <ocr>: 3.08 tok/s

Caveats:
- First run downloads weights; subsequent runs are faster.
- If you hit MPS/dtype errors, set DTYPE=float32 and retry.
- Consider setting PYTORCH_MPS_HIGH_WATERMARK_RATIO=0.0 for lower memory pressure.
