import os
import sys
import json
import base64
from io import BytesIO
from PIL import Image


def _maybe_downscale(img: Image.Image) -> Image.Image:
    try:
        max_side = int(os.getenv("IMAGE_MAX_SIDE", "1600"))
    except Exception:
        max_side = 1600
    w, h = img.size
    side = max(w, h)
    if side <= max_side:
        return img
    scale = max_side / float(side)
    return img.resize((int(w * scale), int(h * scale)), Image.BICUBIC)


def _default_prompt(base_text: str) -> str:
    return (
        "Below is an image of a document page along with its dimensions. "
        "Simply return the markdown representation of this document, presenting tables in markdown format as they naturally appear.\n"
        "If the document contains images, use a placeholder like dummy.png for each image.\n"
        "Your final output must be in JSON format with a single key `natural_text` containing the response.\n"
        f"RAW_TEXT_START\n{base_text}\nRAW_TEXT_END"
    )


def run_ocr(image_path: str) -> dict:
    try:
        from openai import OpenAI
    except Exception as e:
        return {"error": f"openai package not installed: {e}"}

    # Encode image to base64 PNG
    try:
        img = Image.open(image_path).convert("RGB")
    except Exception as e:
        return {"error": f"failed to open image: {e}"}
    img = _maybe_downscale(img)
    buff = BytesIO()
    img.save(buff, format="PNG")
    b64 = base64.b64encode(buff.getvalue()).decode("utf-8")

    base_text = os.getenv("BASE_TEXT", "")
    prompt = _default_prompt(base_text)

    base_url = os.getenv("BASE_URL", "https://api.opentyphoon.ai/v1")
    api_key = os.getenv("TYPHOON_OCR_API_KEY") or os.getenv("OPENAI_API_KEY")
    if not api_key:
        return {"error": "Missing API key: set TYPHOON_OCR_API_KEY or OPENAI_API_KEY"}

    client = OpenAI(base_url=base_url, api_key=api_key)
    max_tokens = int(os.getenv("MAX_NEW_TOKENS", "8192"))
    temperature = float(os.getenv("TEMPERATURE", "0.1"))
    top_p = float(os.getenv("TOP_P", "0.6"))

    try:
        resp = client.chat.completions.create(
            model=os.getenv("TYPHOON_MODEL_NAME", "typhoon-ocr-preview"),
            messages=[
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": prompt},
                        {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{b64}"}},
                    ],
                }
            ],
            max_tokens=max_tokens,
            temperature=temperature,
            top_p=top_p,
            extra_body={"repetition_penalty": 1.2},
        )
    except Exception as e:
        return {"error": str(e)}

    text = resp.choices[0].message.content if resp.choices else ""
    cleaned = (text or "").strip()
    try:
        data = json.loads(cleaned)
        if not isinstance(data, dict) or "natural_text" not in data:
            data = {"natural_text": cleaned}
    except Exception:
        data = {"natural_text": cleaned}
    return data


def main():
    # CLI: python run_typhoon_ocr.py [image_path]
    img_path = sys.argv[1] if len(sys.argv) > 1 else os.getenv("IMAGE", "1.jpg")
    if not os.path.exists(img_path):
        print(json.dumps({"error": f"Image not found: {img_path}"}, ensure_ascii=False))
        sys.exit(1)

    result = run_ocr(img_path)

    # Save and print a single JSON
    out_path = os.getenv("OUTPUT_JSON", "output_ocr_parsed.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)

    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
