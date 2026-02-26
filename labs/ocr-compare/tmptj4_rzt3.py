
            import json
            from paddleocr import PaddleOCR
            img_path = r"/Users/hero/Documents/work/ocr-compare/tmpwq9qsxpx.png"
            ocr = PaddleOCR(lang='th', use_angle_cls=True)
            res = ocr.ocr(img_path)
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
            print(json.dumps({'text':'
'.join(lines),'boxes':boxes,'confs':confs}, ensure_ascii=False))
