from typing import Dict, Any, Callable
import time
import Levenshtein as lev
from jiwer import wer
from pythainlp import word_tokenize
from .utils import normalize_text, is_thai_text


def timeit(fn: Callable, *args, **kwargs):
    t0 = time.perf_counter()
    out = fn(*args, **kwargs)
    dt = time.perf_counter() - t0
    return out, dt


def cer(ref: str, hyp: str) -> float:
    ref_n = normalize_text(ref)
    hyp_n = normalize_text(hyp)
    if len(ref_n) == 0:
        return 0.0 if len(hyp_n) == 0 else 1.0
    return lev.distance(ref_n, hyp_n) / float(len(ref_n))


def _thai_aware_wer(ref: str, hyp: str) -> float:
    ref_n = normalize_text(ref)
    hyp_n = normalize_text(hyp)
    if not ref_n and not hyp_n:
        return 0.0
    # Tokenize with PyThaiNLP if Thai detected; else whitespace split
    if is_thai_text(ref_n) or is_thai_text(hyp_n):
        ref_toks = word_tokenize(ref_n, engine="newmm")
        hyp_toks = word_tokenize(hyp_n, engine="newmm")
        ref_s = " ".join(ref_toks)
        hyp_s = " ".join(hyp_toks)
        return wer(ref_s, hyp_s)
    else:
        return wer(ref_n, hyp_n)


def compute_metrics(ref: str, hyp: str) -> Dict[str, Any]:
    return {
        "cer": cer(ref, hyp),
        "wer": _thai_aware_wer(ref, hyp),
    }
from typing import Dict
import time
import numpy as np
from jiwer import wer
import Levenshtein as lev
import unicodedata
from typing import List

try:
    from pythainlp.tokenize import word_tokenize
except Exception:
    word_tokenize = None


def normalize_text(s: str) -> str:
    if s is None:
        return ""
    s = unicodedata.normalize("NFKC", s)
    s = s.replace("\r", "\n")
    # remove excessive whitespace; Thai doesn't use spaces between words
    s = "".join(ch for ch in s if not ch.isspace())
    return s


def cer(ref: str, hyp: str) -> float:
    # Character Error Rate using Levenshtein distance
    ref = normalize_text(ref)
    hyp = normalize_text(hyp)
    if not ref:
        return 0.0 if not hyp else 1.0
    return lev.distance(ref, hyp) / len(ref)


def compute_metrics(ref: str, hyp: str) -> Dict[str, float]:
    cer_val = cer(ref, hyp)
    # WER: Thai tokenization if available
    if word_tokenize is not None:
        ref_tokens: List[str] = word_tokenize(ref, engine="newmm")
        hyp_tokens: List[str] = word_tokenize(hyp, engine="newmm")
        # Join with spaces for jiwer
        ref_join = " ".join(ref_tokens)
        hyp_join = " ".join(hyp_tokens)
        try:
            wer_val = wer(ref_join, hyp_join)
        except Exception:
            wer_val = 1.0
    else:
        wer_val = wer(ref, hyp)
    return {"cer": cer_val, "wer": wer_val}


def timeit(fn, *args, **kwargs):
    t0 = time.perf_counter()
    out = fn(*args, **kwargs)
    dt = time.perf_counter() - t0
    return out, dt
