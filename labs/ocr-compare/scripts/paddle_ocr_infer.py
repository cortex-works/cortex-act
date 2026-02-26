import argparse, json, warnings, os
# Safer defaults for CPU-only envs to avoid illegal instruction with MKLDNN
os.environ.setdefault("FLAGS_use_mkldnn", "false")
os.environ.setdefault("OMP_NUM_THREADS", "1")
os.environ.setdefault("MKL_NUM_THREADS", "1")
os.environ.setdefault("CPU_NUM", "1")
from paddleocr import PaddleOCR
warnings.filterwarnings("ignore", category=DeprecationWarning)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--image', required=True)
    ap.add_argument('--out', default='/work/outputs/paddle_out.json')
    args = ap.parse_args()
    try:
        # Prefer Thai; if unsupported, fallback to a multilingual set that covers Latin and basic symbols
        lang = 'th'
        try:
            ocr = PaddleOCR(lang=lang, use_textline_orientation=True)
        except TypeError:
            ocr = PaddleOCR(lang=lang, use_angle_cls=True)
    except Exception as e:
        # Fallback to latin or en
        try:
            try:
                ocr = PaddleOCR(lang='latin', use_textline_orientation=True)
            except TypeError:
                ocr = PaddleOCR(lang='latin', use_angle_cls=True)
        except Exception:
            try:
                try:
                    ocr = PaddleOCR(lang='en', use_textline_orientation=True)
                except TypeError:
                    ocr = PaddleOCR(lang='en', use_angle_cls=True)
            except Exception:
                raise
        res = ocr.ocr(args.image)
        lines, boxes, confs = [], [], []
        for page in res:
            for item in page:
                if isinstance(item,(list,tuple)) and len(item)==2:
                    box, tc = item
                    if isinstance(tc,(list,tuple)) and len(tc)==2:
                        txt, conf = tc
                    else:
                        txt, conf = str(tc), 0.0
                elif isinstance(item, dict):
                    box = item.get('points') or item.get('box')
                    txt = item.get('text','')
                    conf = float(item.get('score',0.0))
                else:
                    continue
                lines.append(txt); boxes.append(box); confs.append(float(conf))
        out = {'text':'\n'.join(lines),'boxes':boxes,'confs':confs}
    except Exception as e:
        out = {'error': str(e), 'text': '', 'boxes': [], 'confs': []}
    try:
        with open(args.out, 'w', encoding='utf-8') as f:
            json.dump(out, f, ensure_ascii=False)
    except Exception:
        pass
    print(json.dumps(out, ensure_ascii=False))

if __name__=='__main__':
    main()
