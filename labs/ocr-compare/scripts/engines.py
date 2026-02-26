from typing import Dict, Any
import os
import cv2
import numpy as np


def run_tesseract(image: np.ndarray) -> Dict[str, Any]:
    import pytesseract
    config = "--oem 1 --psm 6"
    text = pytesseract.image_to_string(image, lang="tha+eng", config=config)
    # confidences may not be available; skip advanced parse for simplicity
    return {"text": text, "engine": "tesseract"}


def run_easyocr(image: np.ndarray) -> Dict[str, Any]:
    import easyocr
    reader = easyocr.Reader(["th", "en"], gpu=False)
    result = reader.readtext(image, detail=1, paragraph=True)
    # Concatenate lines
    lines = [r[1] for r in result]
    text = "\n".join(lines)
    return {"text": text, "engine": "easyocr"}


def run_thai_trocr(image: np.ndarray) -> Dict[str, Any]:
    from transformers import VisionEncoderDecoderModel, TrOCRProcessor
    from PIL import Image
    import torch
    # Prefer an available Thai TrOCR; fallback to microsoft/trocr-base-handwritten
    model_id_candidates = [
        "openthaigpt/thai-trocr",
        "suchut/thaitrocr-base-handwritten-beta2",
        "microsoft/trocr-base-handwritten",
    ]
    model = None
    processor = None
    last_err = None
    for mid in model_id_candidates:
        try:
            model = VisionEncoderDecoderModel.from_pretrained(mid)
            processor = TrOCRProcessor.from_pretrained(mid)
            break
        except Exception as e:  # noqa
            last_err = e
            model = None
            processor = None
    if model is None or processor is None:
        raise RuntimeError(f"Failed to load TrOCR models: {last_err}")

    img_pil = Image.fromarray(image if image.ndim == 3 else cv2.cvtColor(image, cv2.COLOR_GRAY2BGR))
    pixel_values = processor(images=img_pil, return_tensors="pt").pixel_values
    with torch.no_grad():
        generated_ids = model.generate(pixel_values)
    text = processor.batch_decode(generated_ids, skip_special_tokens=True)[0]
    return {"text": text, "engine": "thai-trocr"}


def run_paddleocr(image: np.ndarray) -> Dict[str, Any]:
    try:
        from paddleocr import PaddleOCR
    except Exception as e:  # paddle not installed or incompatible
        return {"text": "", "engine": "paddleocr", "error": f"PaddleOCR not available: {e}"}

    # Try Thai, fallback to latin/en depending on wheel support
    for lang in ("th", "latin", "en"):
        try:
            ocr = PaddleOCR(lang=lang, use_angle_cls=True, show_log=False)
            result = ocr.ocr(image, cls=True)
            lines = []
            if result and isinstance(result, list):
                for page in result:
                    if not page:
                        continue
                    for line in page:
                        if isinstance(line, list) and len(line) >= 2:
                            txt = line[1][0]
                            lines.append(txt)
            text = "\n".join(lines)
            return {"text": text, "engine": f"paddleocr({lang})"}
        except AssertionError as e:
            # unsupported language
            last_err = str(e)
            continue
        except Exception as e:
            return {"text": "", "engine": "paddleocr", "error": str(e)}
    return {"text": "", "engine": "paddleocr", "error": "No supported language"}
from typing import Dict, Tuple
import os
import json
import platform
import time
import subprocess
import numpy as np
import cv2

from PIL import Image
import torch

# Utilities

def to_pil(gray_or_bgr: np.ndarray) -> Image.Image:
    if len(gray_or_bgr.shape) == 2:
        return Image.fromarray(gray_or_bgr)
    return Image.fromarray(cv2.cvtColor(gray_or_bgr, cv2.COLOR_BGR2RGB))


def run_tesseract(img: np.ndarray) -> Tuple[str, Dict]:
    try:
        import pytesseract
    except Exception as e:
        raise RuntimeError("pytesseract not available; install Tesseract + python binding") from e
    cfg = "--oem 1 --psm 6 -l tha+eng"
    data = pytesseract.image_to_data(img, config=cfg, output_type=pytesseract.Output.DICT)
    text = pytesseract.image_to_string(img, config=cfg)
    confs = [c for c in data.get("conf", []) if isinstance(c, (int, float)) and c >= 0]
    meta = {"mean_conf": float(np.mean(confs)) if confs else None, "psm": 6, "oem": 1, "lang": "tha+eng"}
    return text.strip(), meta


def run_easyocr(img: np.ndarray) -> Tuple[str, Dict]:
    try:
        import easyocr
    except Exception as e:
        raise RuntimeError("easyocr not installed") from e
    # certs on macOS venv
    try:
        import certifi, os as _os
        _os.environ.setdefault("SSL_CERT_FILE", certifi.where())
    except Exception:
        pass
    reader = easyocr.Reader(["th", "en"], gpu=torch.cuda.is_available())
    result = reader.readtext(img, detail=1, paragraph=False)
    if not result:
        return "", {"boxes": [], "confs": []}
    try:
        text = "\n".join([r[1] for r in result])
        confs = [float(r[2]) for r in result]
        boxes = [r[0] for r in result]
    except Exception:
        text = "\n".join([str(r) for r in result])
        confs, boxes = [], []
    meta = {"boxes": boxes, "confs": confs}
    return text.strip(), meta


_paddle_singleton = None


def _paddleocr_via_docker(img: np.ndarray) -> Tuple[str, Dict]:
    # Write stable path to avoid remount churn
    os.makedirs("outputs", exist_ok=True)
    host_img = os.path.join(os.getcwd(), "outputs", "paddle_input.png")
    cv2.imwrite(host_img, img)
    container_img = "/work/outputs/paddle_input.png"

    name = os.environ.get("PADDLE_DOCKER_NAME", "ocr_paddle_runtime")
    host_machine = platform.machine().lower()
    platform_override = os.environ.get("PADDLE_DOCKER_PLATFORM")
    # Try multiple platforms to avoid mismatch on Apple Silicon
    if platform_override:
        platform_candidates = [platform_override]
    elif "arm64" in host_machine or "aarch64" in host_machine:
        platform_candidates = ["linux/arm64/v8"]
    else:
        platform_candidates = ["linux/amd64"]

    def run_cmd(args):
        return subprocess.run(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)

    # Choose images to try (preference: paddleocr image with OCR preinstalled)
    image_override = os.environ.get("PADDLE_DOCKER_IMAGE")
    images = [image_override] if image_override else [
        # Prefer our pinned local image tag
        "ocr-paddleocr:arm64",
        # Repo-based image
        "ocr-paddleocr-repo:arm64",
        # Fallback: python base
        "python:3.10-slim",
    ]

    # Create container if missing (or wrong image)
    recreate = False
    insp = run_cmd(["docker", "inspect", "-f", "{{.Config.Image}}", name])
    if insp.returncode != 0:
        recreate = True
    else:
        if insp.stdout.strip() not in images:
            run_cmd(["docker", "rm", "-f", name])
            recreate = True
    if recreate:
        # Try to build local pinned image if requested tag not present
        if "ocr-paddleocr:arm64" in images and run_cmd(["docker", "image", "inspect", "ocr-paddleocr:arm64"]).returncode != 0:
            dockerfile = os.path.join("docker", "Dockerfile.paddleocr-arm64")
            if os.path.exists(dockerfile):
                build = run_cmd(["docker", "build", "-t", "ocr-paddleocr:arm64", "-f", dockerfile, "."])
                if build.returncode != 0:
                    # Don't fail outright; continue to fallbacks
                    pass
        if "ocr-paddleocr-repo:arm64" in images and run_cmd(["docker", "image", "inspect", "ocr-paddleocr-repo:arm64"]).returncode != 0:
            dockerfile = os.path.join("docker", "Dockerfile.paddleocr-repo-arm64")
            if os.path.exists(dockerfile):
                build = run_cmd(["docker", "build", "-t", "ocr-paddleocr-repo:arm64", "-f", dockerfile, "."])
                if build.returncode != 0:
                    pass
        last_err = None
        for img in images:
            for plat in platform_candidates:
                # Prepare setup: install system libs and proper paddle wheels
                if img.startswith("ocr-paddleocr:") or img.startswith("ocr-paddleocr-repo:"):
                    setup_cmd = "tail -f /dev/null"
                else:
                    if plat.startswith("linux/arm64"):
                        pip_paddle = "python3 -m pip install -q --no-cache-dir paddlepaddle==2.6.1 -f https://www.paddlepaddle.org.cn/whl/linux/aarch64/stable.html"
                    else:
                        pip_paddle = "python3 -m pip install -q --no-cache-dir paddlepaddle==2.6.1"
                    setup_cmd = (
                        "apt-get update >/dev/null 2>&1 && apt-get install -y -qq libgl1 libglib2.0-0 libsm6 libxrender1 libxext6 libgomp1 >/dev/null 2>&1 || true; "
                        "python3 -m pip install -q --no-cache-dir -U pip setuptools wheel >/dev/null 2>&1 || true; "
                        f"{pip_paddle} >/dev/null 2>&1 || true; "
                        "python3 -m pip install -q --no-cache-dir paddleocr==2.6.0.1 >/dev/null 2>&1 || true; "
                        "tail -f /dev/null"
                    )
                args = [
                    "docker", "run", "-d", "--name", name, "--platform", plat,
                    "-v", f"{os.getcwd()}:/work", "-w", "/work",
                    img,
                    "bash", "-lc", setup_cmd,
                ]
                create = run_cmd(args)
                if create.returncode == 0:
                    last_err = None
                    image_used = img
                    break
                last_err = create.stdout
                run_cmd(["docker", "rm", "-f", name])
            if last_err is None:
                break
        if last_err is not None:
            raise RuntimeError(f"paddleocr docker failed to start: {last_err}")
        time.sleep(1.5)
    else:
        # Start if stopped
        state = run_cmd(["docker", "inspect", "-f", "{{.State.Running}}", name])
        if state.stdout.strip() != "true":
            start = run_cmd(["docker", "start", name])
            if start.returncode != 0:
                raise RuntimeError(f"paddleocr docker failed to start existing container: {start.stdout}")

    # Ensure libs and paddleocr are installed (idempotent); prefer paddleocr CLI
    prep = run_cmd(["docker", "exec", name, "bash", "-lc",
                    "apt-get update >/dev/null 2>&1 && apt-get install -y -qq libgl1 libglib2.0-0 libsm6 libxrender1 libxext6 >/dev/null 2>&1 || true; "
                    "python3 -m pip install -q --no-cache-dir -U pip setuptools wheel >/dev/null 2>&1 || true; "
                    # Pin versions to avoid ABI conflicts and paddlex deps
                    "python3 -m pip install -q --no-cache-dir numpy==1.23.5 opencv-python==4.6.0.66 imgaug==0.4.0 scikit-image==0.21.0 pillow==10.4.0 >/dev/null 2>&1 || true; "
                    "python3 -m pip install -q --no-cache-dir paddlepaddle==2.6.1 -f https://www.paddlepaddle.org.cn/whl/linux/aarch64/stable.html >/dev/null 2>&1 || true; "
                    "python3 -m pip install -q --no-cache-dir paddleocr==2.6.0.1 >/dev/null 2>&1 || true"])
    def _exec_and_read() -> Tuple[str, Dict]:
        # remove stale json if any
        try:
            os.remove(os.path.join(os.getcwd(), "outputs", "paddle_out.json"))
        except FileNotFoundError:
            pass
        # Prefer pip-based helper
        container_image = run_cmd(["docker", "inspect", "-f", "{{.Config.Image}}", name]).stdout.strip()
        # Use pip-based helper uniformly; it will install/download models as needed
        runner = f"python3 -u scripts/paddle_ocr_infer.py --image {container_img} --out /work/outputs/paddle_out.json"
        exec_run_local = run_cmd(["docker", "exec", name, "bash", "-lc", runner])
        out_json_path_local = os.path.join(os.getcwd(), "outputs", "paddle_out.json")
        data_local = None
        if os.path.exists(out_json_path_local):
            with open(out_json_path_local, "r", encoding="utf-8") as f:
                data_local = json.load(f)
        else:
            if exec_run_local.stdout:
                last = exec_run_local.stdout.strip().splitlines()[-1]
                data_local = json.loads(last)
        if data_local is None:
            raise RuntimeError(f"paddleocr docker failed (exec): {exec_run_local.stdout or 'no stdout'}")
        if isinstance(data_local, dict) and data_local.get("error"):
            raise RuntimeError(f"paddleocr in-container error: {data_local['error']}")
        # Accept either structured or raw output
        if isinstance(data_local, dict) and "text" in data_local:
            return data_local.get("text", "").strip(), {"boxes": data_local.get("boxes", []), "confs": data_local.get("confs", []), "runtime": f"docker:{name}"}
        raw = data_local.get("raw", "") if isinstance(data_local, dict) else ""
        return raw.strip(), {"runtime": f"docker:{name}", "note": "raw output"}

    return _exec_and_read()


def run_paddleocr(img: np.ndarray) -> Tuple[str, Dict]:
    global _paddle_singleton
    # Try local Python first
    try:
        if _paddle_singleton is None:
            from paddleocr import PaddleOCR  # type: ignore
            _paddle_singleton = PaddleOCR(lang="th", use_angle_cls=True)
        ocr = _paddle_singleton
        result = ocr.ocr(img)
        lines, confs, boxes = [], [], []
        for page in result:
            for item in page:
                if isinstance(item, (list, tuple)) and len(item) == 2:
                    box, tc = item
                    if isinstance(tc, (list, tuple)) and len(tc) == 2:
                        txt, conf = tc
                    else:
                        txt, conf = str(tc), 0.0
                elif isinstance(item, dict):
                    box = item.get("points") or item.get("box")
                    txt = item.get("text", "")
                    conf = float(item.get("score", 0.0))
                else:
                    continue
                lines.append(txt); confs.append(float(conf)); boxes.append(box)
        text = "\n".join(lines)
        return text.strip(), {"boxes": boxes, "confs": confs, "runtime": "local"}
    except Exception:
        # Use persistent Docker container (idempotent; no duplicates)
        return _paddleocr_via_docker(img)


_trocr_model = None
_trocr_processor = None


def load_thai_trocr():
    try:
        from transformers import TrOCRProcessor, VisionEncoderDecoderModel
    except Exception as e:
        raise RuntimeError("transformers not installed for TrOCR") from e
    model_id = os.environ.get("THAI_TROCR_MODEL", "microsoft/trocr-base-stage1")
    device = "cuda" if torch.cuda.is_available() else "cpu"
    processor = TrOCRProcessor.from_pretrained(model_id)
    model = VisionEncoderDecoderModel.from_pretrained(model_id).to(device)
    return processor, model


def run_thai_trocr(img: np.ndarray) -> Tuple[str, Dict]:
    global _trocr_model, _trocr_processor
    if _trocr_model is None or _trocr_processor is None:
        _trocr_processor, _trocr_model = load_thai_trocr()
    pil = to_pil(img).convert("RGB")
    device = next(_trocr_model.parameters()).device
    pixel_values = _trocr_processor(images=pil, return_tensors="pt").pixel_values.to(device)
    with torch.no_grad():
        generated_ids = _trocr_model.generate(pixel_values, max_length=512)
    text = _trocr_processor.batch_decode(generated_ids, skip_special_tokens=True)[0]
    meta = {"device": str(device), "model": getattr(_trocr_model, "name_or_path", None)}
    return text.strip(), meta


ENGINES = {
    "tesseract": run_tesseract,
    "easyocr": run_easyocr,
    "paddleocr": run_paddleocr,
    "thai_trocr": run_thai_trocr,
}
