import argparse, json, os, subprocess, sys, re, ast

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--image', required=True)
    ap.add_argument('--out', default='/work/outputs/paddle_out.json')
    ap.add_argument('--lang', default='th')
    args = ap.parse_args()
    # Ensure inference models exist (det v3 + Thai rec v3)
    sh = r'''
set -e
cd /opt/PaddleOCR
mkdir -p inference
cd inference
if [ ! -d ch_PP-OCRv3_det_infer ]; then
    echo "Downloading det model...";
    wget -S https://paddleocr.bj.bcebos.com/PP-OCRv3/det/ch_PP-OCRv3_det_infer.tar -O ch_PP-OCRv3_det_infer.tar;
    tar -xf ch_PP-OCRv3_det_infer.tar && rm -f ch_PP-OCRv3_det_infer.tar;
fi
if [ ! -d th_PP-OCRv3_rec_infer ]; then
    echo "Downloading Thai rec model...";
    wget -S https://paddleocr.bj.bcebos.com/PP-OCRv3/rec/th_PP-OCRv3_rec_infer.tar -O th_PP-OCRv3_rec_infer.tar;
    tar -xf th_PP-OCRv3_rec_infer.tar && rm -f th_PP-OCRv3_rec_infer.tar;
fi
if [ ! -f ch_PP-OCRv3_det_infer/model.pdmodel ] || [ ! -f ch_PP-OCRv3_det_infer/model.pdiparams ]; then
    echo "Detector model files missing" >&2; exit 7; fi
if [ ! -f th_PP-OCRv3_rec_infer/model.pdmodel ] || [ ! -f th_PP-OCRv3_rec_infer/model.pdiparams ]; then
    echo "Thai rec model files missing" >&2; exit 8; fi
cd /opt/PaddleOCR
python3 tools/infer/predict_system.py --image_dir {img} --use_angle_cls True \
    --det_model_dir /opt/PaddleOCR/inference/ch_PP-OCRv3_det_infer \
    --rec_model_dir /opt/PaddleOCR/inference/th_PP-OCRv3_rec_infer 2>&1
'''.format(img=args.image)
    p = subprocess.run(['bash','-lc', sh], text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    s = p.stdout
    if p.returncode != 0:
        out = {"error": f"repo runner failed (rc={p.returncode})", "raw": s[-2000:]}
        try:
            with open(args.out,'w',encoding='utf-8') as f:
                json.dump(out,f,ensure_ascii=False)
        except Exception:
            pass
        print(json.dumps(out, ensure_ascii=False))
        sys.exit(p.returncode)
    # Parse "result: [[box], ('text', conf)]" lines
    lines, boxes, confs = [], [], []
    for m in re.finditer(r"result:\s*(\[.*?\])\s*$", s, re.S|re.M):
        try:
            arr = ast.literal_eval(m.group(1))
            # arr could be list of items; normalize
            for item in arr:
                if isinstance(item, (list, tuple)) and len(item) == 2:
                    box, tc = item
                    if isinstance(tc, (list, tuple)) and len(tc) == 2:
                        txt, conf = tc
                    else:
                        txt, conf = str(tc), 0.0
                    lines.append(str(txt))
                    boxes.append(box)
                    try:
                        confs.append(float(conf))
                    except Exception:
                        confs.append(0.0)
        except Exception:
            continue
    out = {"text": "\n".join(lines), "boxes": boxes, "confs": confs, "raw": s}
    try:
        with open(args.out,'w',encoding='utf-8') as f:
            json.dump(out,f,ensure_ascii=False)
    except Exception:
        pass
    print(json.dumps(out, ensure_ascii=False))

if __name__ == '__main__':
    main()
