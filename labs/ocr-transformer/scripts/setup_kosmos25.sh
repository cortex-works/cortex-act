#!/usr/bin/env bash
set -euo pipefail

# KOSMOS-2.5 on Apple Silicon (PyTorch + MPS) setup
# - Creates a Python venv
# - Installs PyTorch with MPS support and Transformers
# - Verifies that MPS is available

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

echo "[1/3] Creating virtual environment at $ROOT/.venv"
python3 -m venv "$ROOT/.venv"
source "$ROOT/.venv/bin/activate"

echo "[2/3] Upgrading pip tooling"
python -m pip install --upgrade pip wheel setuptools

echo "[2/3] Installing core deps (PyTorch + Transformers stack)"
pip install --upgrade torch torchvision torchaudio
pip install --upgrade transformers accelerate pillow requests tokenizers

echo "[3/3] Verifying MPS availability"
python - <<'PY'
import torch, platform
print("Python:", platform.python_version())
print("Torch:", torch.__version__)
print("MPS available:", torch.backends.mps.is_available())
print("MPS built:", torch.backends.mps.is_built())
PY

echo "\nIf 'MPS available: True' is shown above, you're good to go."
