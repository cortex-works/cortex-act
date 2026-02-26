import argparse
import os
import json
from typing import List, Dict, Any
import cv2
from rich import print

from ocrbench.preprocess import preprocess
from ocrbench.metrics import compute_metrics, timeit
from ocrbench.engines import run_tesseract, run_easyocr, run_thai_trocr, run_paddleocr
from ocrbench.utils import ensure_dirs, read_text, write_json, write_text, normalize_text


ENGINE_FUNCS = {
    "tesseract": run_tesseract,
    "easyocr": run_easyocr,
    "trocr": run_thai_trocr,
    "paddle": run_paddleocr,
}


def run_benchmark(image_path: str, engines: List[str], gt_path: str = None, save_debug: bool = False) -> Dict[str, Any]:
    ensure_dirs("outputs", "reports", "debug")
    img = cv2.imread(image_path)
    if img is None:
        raise FileNotFoundError(f"Cannot read image: {image_path}")

    bin_img, pp_meta = preprocess(img)
    if save_debug:
        cv2.imwrite(os.path.join("debug", "preprocessed.jpg"), bin_img)

    gt = read_text(gt_path) if gt_path else None
    results = []

    for eng in engines:
        fn = ENGINE_FUNCS.get(eng)
        if not fn:
            print(f"[yellow]Unknown engine: {eng}[/yellow]")
            continue
        print(f"[cyan]Running {eng}...[/cyan]")
        try:
            out, dt = timeit(fn, bin_img)
        except Exception as e:
            out = {"text": "", "engine": eng, "error": str(e)}
            dt = 0.0
        text = out.get("text", "")
        record = {
            "engine": eng,
            "seconds": dt,
            "text": text,
            "meta": out,
        }
        if gt:
            m = compute_metrics(gt, text)
            record.update(m)
        # Save raw text
        write_text(os.path.join("outputs", f"{eng}.txt"), text)
        results.append(record)

    # Summary
    summary = {
        "image": image_path,
        "engines": engines,
        "preprocess": pp_meta,
        "results": results,
    }
    write_json(os.path.join("reports", "summary.json"), summary)

    # Also a light Markdown summary
    lines = [f"# OCR Benchmark Summary", "", f"Image: {image_path}", ""]
    for r in results:
        line = f"- {r['engine']}: time={r['seconds']:.2f}s"
        if 'cer' in r and 'wer' in r:
            line += f", CER={r['cer']:.3f}, WER={r['wer']:.3f}"
        if r.get('meta', {}).get('error'):
            line += f" (error: {r['meta']['error']})"
        lines.append(line)
    write_text(os.path.join("reports", "summary.md"), "\n".join(lines))
    return summary


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--image", default="1.jpg", help="Input image path")
    p.add_argument("--gt", default="ground_truth.txt", help="Ground-truth text file (optional)")
    p.add_argument("--engines", default="tesseract,easyocr,trocr", help="Comma-separated engines: tesseract,easyocr,trocr,paddle")
    p.add_argument("--save-debug", action="store_true")
    return p.parse_args()


if __name__ == "__main__":
    args = parse_args()
    engines = [e.strip() for e in args.engines.split(",") if e.strip()]
    summary = run_benchmark(args.image, engines, args.gt, args.save_debug)
    print("\n[green]Done. Reports in reports/[/green]")
