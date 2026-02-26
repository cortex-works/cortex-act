import os, sys, json

def main():
    try:
        from typhoon_ocr import ocr_document
    except Exception as e:
        print(json.dumps({"error": f"typhoon_ocr not installed: {e}"}, ensure_ascii=False))
        sys.exit(1)

    img = sys.argv[1] if len(sys.argv) > 1 else os.getenv("IMAGE", "1.jpg")
    base_url = os.getenv("BASE_URL")  # e.g., http://localhost:8000/v1
    api_key = os.getenv("TYPHOON_OCR_API_KEY") or os.getenv("OPENAI_API_KEY") or os.getenv("API_KEY", "no-key")
    try:
        md = ocr_document(img, base_url=base_url, api_key=api_key)
    except Exception as e:
        print(json.dumps({"error": str(e)}, ensure_ascii=False))
        sys.exit(2)
    print(json.dumps({"natural_text": md}, ensure_ascii=False))

if __name__ == "__main__":
    main()
