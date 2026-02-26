#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

PY=${PYTHON:-python3}

if [ ! -d .venv ]; then
  ${PY} -m venv .venv
fi
source .venv/bin/activate

pip install --upgrade pip wheel setuptools

# Core deps: torch with MPS, transformers with Qwen2.5-VL, pillow
pip install --upgrade "torch>=2.3.0" torchvision torchaudio
pip install --upgrade "transformers>=4.45" tokenizers accelerate pillow

# Optional: official typhoon-ocr helpers (anchor text, pdf utils)
pip install --upgrade typhoon-ocr

${PY} - << 'PY'
import torch, sys
print('python:', sys.version.split()[0])
print('torch:', torch.__version__)
print('mps available:', torch.backends.mps.is_available(), 'built:', torch.backends.mps.is_built())
try:
    import transformers as t
    from transformers import Qwen2_5_VLForConditionalGeneration, AutoProcessor
    print('transformers:', t.__version__)
    print('Qwen2_5_VLForConditionalGeneration: OK')
except Exception as e:
    print('transformers import error:', e)
PY

echo "Setup complete. Activate with: source .venv/bin/activate"