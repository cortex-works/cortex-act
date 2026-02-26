import json
import os
import unicodedata
import regex as re
from typing import Optional


def ensure_dirs(*paths: str) -> None:
    for p in paths:
        os.makedirs(p, exist_ok=True)


def read_text(path: str) -> Optional[str]:
    if not path or not os.path.exists(path):
        return None
    with open(path, 'r', encoding='utf-8') as f:
        return f.read()


def write_json(path: str, data) -> None:
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def write_text(path: str, data: str) -> None:
    with open(path, 'w', encoding='utf-8') as f:
        f.write(data)


THAI_RE = re.compile(r"\p{Script=Thai}")


def is_thai_text(s: str) -> bool:
    if not s:
        return False
    return bool(THAI_RE.search(s))


def normalize_text(s: str) -> str:
    if s is None:
        return ""
    s = unicodedata.normalize('NFKC', s)
    # collapse spaces and newlines
    s = re.sub(r"\s+", " ", s, flags=re.MULTILINE).strip()
    return s
