import argparse
import json
import os
from pathlib import Path
from typing import Dict

import cv2
from rich import print

from preprocess import preprocess, save_debug
from engines import ENGINES
from metrics import compute_metrics, timeit


def read_text(path: str) -> str:
    with open(path, "r", encoding="utf-8") as f:
        return f.read().strip()


def write_text(path: str, text: str):
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def to_jsonable(obj):
    import numpy as np
    if obj is None:
        return None
    if isinstance(obj, (str, int, float, bool)):
        return obj
    if isinstance(obj, dict):
        return {str(k): to_jsonable(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [to_jsonable(x) for x in obj]
    if hasattr(obj, "tolist"):
        try:
            return obj.tolist()
        except Exception:
            return str(obj)
    if isinstance(obj, (np.generic,)):
        return obj.item()
    return str(obj)


def main():
    p = argparse.ArgumentParser(description="Run OCR benchmark on a single image")
    p.add_argument("--image", required=True)
    p.add_argument("--ground-truth", required=True)
    p.add_argument("--engines", nargs="*", default=["tesseract", "easyocr", "paddleocr", "thai_trocr"],
                   help="subset of: tesseract easyocr paddleocr thai_trocr")
    p.add_argument("--outdir", default="outputs")
    args = p.parse_args()

    os.makedirs(args.outdir, exist_ok=True)
    os.makedirs("reports", exist_ok=True)

    # Preprocess
    proc = preprocess(args.image)
    save_debug(proc, os.path.join(args.outdir, "preprocessed"))
    # Use deskewed grayscale; some DL OCR models prefer grayscale over hard binarization
    img = proc["deskewed"]

    # ground truth text
    gt = read_text(args.ground_truth)

    summary = {}

    for name in args.engines:
        if name not in ENGINES:
            print(f"[yellow]Skipping unknown engine {name}")
            continue
        fn = ENGINES[name]
        try:
            (text, meta), dt = timeit(fn, img)
        except Exception as e:
            print(f"[red]{name} failed[/red]: {e}")
            summary[name] = {"metrics": {"cer": None, "wer": None}, "time_s": None, "meta": {"error": str(e)}}
            continue
        out_path = os.path.join(args.outdir, f"{name}.txt")
        write_text(out_path, text)
        m = compute_metrics(gt, text)
        summary[name] = {
            "name": name,
            "metrics": m,
            "time_s": dt,
            "meta": to_jsonable(meta),
        }
        print(f"[green]{name}[/green] -> time: {dt:.3f}s, CER={m['cer']:.3f}, WER={m['wer']:.3f}")

    with open("reports/summary.json", "w", encoding="utf-8") as f:
        json.dump(summary, f, ensure_ascii=False, indent=2)

    # Render simple markdown table
    headers = ["Tool", "CER", "WER", "Speed (s)"]
    lines = ["| " + " | ".join(headers) + " |", "|" + "---|"*len(headers)]
    for tool, vals in summary.items():
        cer = vals["metrics"]["cer"]
        wer = vals["metrics"]["wer"]
        t = vals["time_s"]
        def fmt(x):
            return "-" if x is None else f"{x:.3f}"
        lines.append(f"| {tool} | {fmt(cer)} | {fmt(wer)} | {fmt(t)} |")
    md = "\n".join(lines)
    with open("reports/summary.md", "w", encoding="utf-8") as f:
        f.write(md)

    print("\nSaved reports to reports/summary.{json,md}")


if __name__ == "__main__":
    main()
