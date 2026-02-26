from typing import Dict, Any
import cv2
import numpy as np


def run_tesseract(image: np.ndarray) -> Dict[str, Any]:
    import pytesseract
    config = "--oem 1 --psm 6"
    text = pytesseract.image_to_string(image, lang="tha+eng", config=config)
    return {"text": text, "engine": "tesseract"}


def run_easyocr(image: np.ndarray) -> Dict[str, Any]:
    import easyocr
    reader = easyocr.Reader(["th", "en"], gpu=False)
    result = reader.readtext(image, detail=1, paragraph=True)
    lines = [r[1] for r in result]
    text = "\n".join(lines)
    return {"text": text, "engine": "easyocr"}


def run_thai_trocr(image: np.ndarray) -> Dict[str, Any]:
    from transformers import VisionEncoderDecoderModel, TrOCRProcessor
    from PIL import Image
    import torch
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
    except Exception as e:
        return {"text": "", "engine": "paddleocr", "error": f"PaddleOCR not available: {e}"}

    for lang in ("th", "en", None):
        try:
            if lang is None:
                ocr = PaddleOCR(use_angle_cls=True)
            else:
                ocr = PaddleOCR(lang=lang, use_angle_cls=True)
            result = ocr.ocr(image)
            lines = []
            if result and isinstance(result, list):
                for page in result:
                    if not page:
                        continue
                    for line in page:
                        if isinstance(line, list) and len(line) >= 2:
                            # Handle different result formats
                            if isinstance(line[1], list) and len(line[1]) > 0:
                                txt = line[1][0] if isinstance(line[1][0], str) else str(line[1][0])
                            else:
                                txt = str(line[1])
                            lines.append(txt)
            text = "\n".join(lines)
            return {"text": text, "engine": f"paddleocr({lang})"}
        except Exception as e:
            continue
    return {"text": "", "engine": "paddleocr", "error": "No supported language"}
