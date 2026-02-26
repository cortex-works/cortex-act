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
