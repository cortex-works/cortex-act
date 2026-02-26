from typing import Tuple, Dict, Any
import cv2
import numpy as np
from skimage.filters import threshold_sauvola
import os


def _resize_min_side(img: np.ndarray, min_side: int = 1200) -> np.ndarray:
    h, w = img.shape[:2]
    scale = max(1.0, float(min_side) / float(min(h, w)))
    if scale == 1.0:
        return img
    new_w = int(round(w * scale))
    new_h = int(round(h * scale))
    return cv2.resize(img, (new_w, new_h), interpolation=cv2.INTER_AREA)


def _grayscale(img: np.ndarray) -> np.ndarray:
    if len(img.shape) == 2:
        return img
    return cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)


def _rotate(img: np.ndarray, angle: float) -> np.ndarray:
    h, w = img.shape[:2]
    M = cv2.getRotationMatrix2D((w / 2, h / 2), angle, 1.0)
    return cv2.warpAffine(img, M, (w, h), flags=cv2.INTER_LINEAR, borderMode=cv2.BORDER_REPLICATE)


def _deskew_small_search(gray: np.ndarray, angles=(-5, 5), step=1.0):
    best = gray
    best_score = -1e9
    best_angle = 0.0
    for a in np.arange(angles[0], angles[1] + 1e-6, step):
        rot = _rotate(gray, a)
        proj = np.sum(rot, axis=1)
        score = np.var(proj)
        if score > best_score:
            best_score = score
            best = rot
            best_angle = float(a)
    return best, best_angle


def _sauvola(gray: np.ndarray) -> np.ndarray:
    window = max(15, int(round(min(gray.shape[:2]) * 0.02)) | 1)
    thresh = threshold_sauvola(gray, window_size=window)
    bin_img = (gray > thresh).astype(np.uint8) * 255
    return bin_img


def preprocess(bgr: np.ndarray) -> Tuple[np.ndarray, Dict[str, Any]]:
    meta: Dict[str, Any] = {}
    up = _resize_min_side(bgr, 1200)
    meta["resize"] = True
    gray = _grayscale(up)
    meta["grayscale"] = True
    deskewed, angle = _deskew_small_search(gray, (-5, 5), 1.0)
    meta["deskew_angle"] = angle
    bin_img = _sauvola(deskewed)
    meta["binarized"] = True
    return bin_img, meta
