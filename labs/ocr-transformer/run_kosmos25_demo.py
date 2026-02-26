import os
import sys
import time
from io import BytesIO

import requests
import torch
from PIL import Image
from transformers import AutoProcessor
import warnings
from transformers import logging as hf_logging
warnings.filterwarnings("ignore")
hf_logging.set_verbosity_error()
os.environ.setdefault("HF_HUB_DISABLE_PROGRESS_BARS", "1")
try:
    # If available in your transformers version
    from transformers import Kosmos2_5ForConditionalGeneration  # type: ignore
except Exception:  # pragma: no cover
    Kosmos2_5ForConditionalGeneration = None  # type: ignore


REPO = os.environ.get("KOSMOS25_REPO", "microsoft/kosmos-2.5")


def choose_device_dtype():
    use_mps = torch.backends.mps.is_available()
    # Allow override via env
    env_dev = os.environ.get("DEVICE", "").lower()
    if env_dev in {"mps", "gpu"} and use_mps:
        device = "mps"
    elif env_dev == "cpu":
        device = "cpu"
    else:
        device = "mps" if use_mps else "cpu"
    # Prefer bfloat16 on newer Apple Silicon (M3/M4), allow override via env
    env_dtype = os.environ.get("DTYPE", "").lower()
    if env_dtype in {"float32", "fp32"}:
        dtype = torch.float32
    elif env_dtype in {"bfloat16", "bf16"}:
        dtype = torch.bfloat16
    elif env_dtype in {"float16", "fp16"}:
        dtype = torch.float16
    else:
        if device == "mps":
            # On M3/M4, bfloat16 is typically supported and stable; fall back to fp32 otherwise
            dtype = torch.bfloat16
        else:
            dtype = torch.float32
    return device, dtype


def load_image(url_or_path: str) -> Image.Image:
    if url_or_path.startswith("http://") or url_or_path.startswith("https://"):
        resp = requests.get(url_or_path, timeout=60)
        resp.raise_for_status()
        return Image.open(BytesIO(resp.content)).convert("RGB")
    return Image.open(url_or_path).convert("RGB")


def maybe_downscale(image: Image.Image) -> Image.Image:
    # Downscale longest side to at most IMAGE_MAX_SIDE (default 960) to speed up
    try:
        max_side = int(os.environ.get("IMAGE_MAX_SIDE", "960"))
    except Exception:
        max_side = 960
    w, h = image.size
    s = max(w, h)
    if s <= max_side:
        return image
    scale = max_side / float(s)
    new_w, new_h = int(round(w * scale)), int(round(h * scale))
    return image.resize((new_w, new_h), Image.BILINEAR)


def main():
    # Allow overriding the sample via env/argv
    default_url = (
        "https://huggingface.co/microsoft/kosmos-2.5/resolve/main/receipt_00008.png"
    )
    local_default = os.path.join(os.path.dirname(__file__), "shipping_label_02.png")
    preferred = local_default if os.path.exists(local_default) else default_url
    image_src = os.environ.get("IMAGE", sys.argv[1] if len(sys.argv) > 1 else preferred)

    device, dtype = choose_device_dtype()
    if os.environ.get("VERBOSE") == "1":
        print(f"Device: {device} | DType: {dtype}", file=sys.stderr)

    # Load model and processor; enforce SDPA to avoid FlashAttention on Mac
    t0 = time.time()
    model = None
    first_err = None
    if Kosmos2_5ForConditionalGeneration is not None:
        try:
            model = Kosmos2_5ForConditionalGeneration.from_pretrained(
                REPO,
                torch_dtype=dtype,
                attn_implementation="sdpa",
            )
        except Exception as e:
            first_err = e
    if model is None:
        # Fallback: explicit config load with trust_remote_code
        from transformers import AutoConfig, AutoModelForCausalLM
        try:
            cfg = AutoConfig.from_pretrained(REPO, trust_remote_code=True)
            try:
                model = AutoModelForCausalLM.from_pretrained(
                    REPO,
                    config=cfg,
                    torch_dtype=dtype,
                    trust_remote_code=True,
                    attn_implementation="sdpa",
                )
            except TypeError:
                model = AutoModelForCausalLM.from_pretrained(
                    REPO,
                    config=cfg,
                    torch_dtype=dtype,
                    trust_remote_code=True,
                )
        except Exception as e:
            if first_err is not None:
                raise RuntimeError(f"Failed to load model. Native err: {first_err}; Auto fallback err: {e}")
            raise

    processor = AutoProcessor.from_pretrained(REPO, trust_remote_code=True, use_fast=True)
    model.to(device)
    model.eval()
    t1 = time.time()

    image = maybe_downscale(load_image(image_src))

    def run(prompt: str, max_new_tokens=224):
        # Prepare inputs
        inputs = processor(text=prompt, images=image, return_tensors="pt")
        # keep height/width for OCR scaling, but also pass-through
        height = inputs.get("height", None)
        width = inputs.get("width", None)
        raw_w, raw_h = image.size
        if torch.is_tensor(height):
            try:
                height = int(height.item())
            except Exception:
                height = None
        if torch.is_tensor(width):
            try:
                width = int(width.item())
            except Exception:
                width = None
        # To device
        tens = {k: (v.to(device) if torch.is_tensor(v) else v) for k, v in inputs.items()}
        if "flattened_patches" in tens and tens["flattened_patches"].dtype != dtype:
            tens["flattened_patches"] = tens["flattened_patches"].to(dtype)

        # Generate
        with torch.inference_mode():
            g0 = time.time()
            out_ids = model.generate(
                **tens,
                max_new_tokens=max_new_tokens,
                do_sample=False,
                use_cache=True,
            )
            g1 = time.time()

        # Decode (both raw and cleaned to avoid losing tags)
        text_raw = processor.batch_decode(out_ids, skip_special_tokens=False)[0]
        text_clean = processor.batch_decode(out_ids, skip_special_tokens=True)[0]

        # Optional OCR post-processing to extract bbox-tagged lines
        ocr_lines = None
        if prompt.strip() == "<ocr>":
            import re
            y = text_raw.replace(prompt, "")
            if height and width:
                scale_h = raw_h / float(height)
                scale_w = raw_w / float(width)
            else:
                scale_h = scale_w = 1.0

            pattern = r"<bbox><x_\d+><y_\d+><x_\d+><y_\d+></bbox>"
            bboxs_raw = re.findall(pattern, y)
            lines = re.split(pattern, y)[1:]
            bboxs = [re.findall(r"\d+", i) for i in bboxs_raw]
            bboxs = [[int(j) for j in i] for i in bboxs]
            formatted = []
            for i in range(min(len(lines), len(bboxs))):
                x0, y0, x1, y1 = bboxs[i]
                if not (x0 >= x1 or y0 >= y1):
                    x0 = int(x0 * scale_w)
                    y0 = int(y0 * scale_h)
                    x1 = int(x1 * scale_w)
                    y1 = int(y1 * scale_h)
                    text_line = lines[i].strip()
                    if text_line:
                        formatted.append(f"{x0},{y0},{x1},{y0},{x1},{y1},{x0},{y1},{text_line}")
            ocr_lines = "\n".join(formatted)

        # Token throughput estimate
        new_tokens = out_ids.shape[-1] - tens["input_ids"].shape[-1]
        dt = max(1e-6, g1 - g0)
        tps = new_tokens / dt

        # Parse into a simple structured summary for shipping labels and return JSON
        result_json = None
        if ocr_lines is not None:
            try:
                # Parse CSV lines into records
                lines = []
                for ln in ocr_lines.splitlines():
                    parts = ln.split(",")
                    if len(parts) < 9:
                        continue
                    x0,y0,x1,_,_,y1,_,_,text = parts[0],parts[1],parts[2],parts[3],parts[4],parts[5],parts[6],parts[7],",".join(parts[8:])
                    lines.append({
                        "x0": int(x0), "y0": int(y0), "x1": int(x1), "y1": int(y1), "text": text.strip()
                    })
                lines.sort(key=lambda r: (r["y0"], r["x0"]))

                structured = {"ship_to": [], "from": [], "details": {}, "tracking": None}
                left_mid = raw_w / 2.0
                label_names = ["ORDER ID", "WEIGHT", "DIMENSIONS", "SHIPPING DATE", "REMARKS"]

                # Determine top of details table
                y_thresh = next((r["y0"] for r in lines if any(r["text"].upper().startswith(l) for l in label_names)), raw_h)

                # Fill ship_to/from headers
                for r in lines:
                    t = r["text"]
                    if r["y0"] >= y_thresh:
                        continue
                    up = t.upper()
                    if up.startswith("SHIP TO") or up.startswith("FROM") or up.startswith("TRACK"):
                        continue
                    (structured["ship_to"] if r["x0"] < left_mid else structured["from"]).append(t)

                # Tracking
                for r in lines:
                    t = r["text"].strip()
                    if t.upper().startswith("TRACK"):
                        structured["tracking"] = t

                # Pair labels to nearest right-side values
                def nearest_value(label_y, label_x):
                    best, best_dy = None, 1e9
                    for r in lines:
                        t = r["text"].strip()
                        up = t.upper()
                        if up.startswith(tuple(label_names)) or up.startswith(("SHIP TO", "FROM", "TRACK")):
                            continue
                        if r["x0"] < label_x:
                            continue
                        dy = abs(r["y0"] - label_y)
                        if dy < best_dy and (label_y - 40) <= r["y0"] <= (label_y + 60):
                            best, best_dy = t, dy
                    return best

                for name in label_names:
                    for r in lines:
                        if r["text"].upper().startswith(name):
                            val = nearest_value(r["y0"], r["x0"])
                            if val:
                                structured["details"][name] = val
                            break

                import json
                result_json = structured
            except Exception:
                result_json = None

        # Return tokens/sec and the structured json
        return tps, result_json

    def pick_tokens(default_val: int, key: str):
        v = os.environ.get(key)
        if v and v.isdigit():
            return int(v)
        return default_val

    # Per-prompt token overrides
    default_max = int(os.environ.get("MAX_NEW_TOKENS", "0") or 0)
    if default_max <= 0:
        default_max = 224
    max_md = pick_tokens(default_max, "MAX_NEW_TOKENS_MD")
    max_ocr = pick_tokens(default_max, "MAX_NEW_TOKENS_OCR")

    # OCR-only fast path
    _, parsed = run("<ocr>", max_new_tokens=max_ocr)

    # Emit a single JSON to stdout and file
    import json
    out_json = parsed or {"error": "parsing_failed"}
    print(json.dumps(out_json, indent=2))

    # Write a short report next to this script
    # Persist a single JSON artifact
    out_path = os.path.join(os.path.dirname(__file__), "output_ocr_parsed.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out_json, f, indent=2)
    # Intentionally no extra prints; stdout already has the JSON


if __name__ == "__main__":
    main()
