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


def _deskew_small_search(gray: np.ndarray, angles=(-5, 5), step=1.0) -> Tuple[np.ndarray, float]:
    best = gray
    best_score = -1e9
    best_angle = 0.0
    for a in np.arange(angles[0], angles[1] + 1e-6, step):
        rot = _rotate(gray, a)
        # project horizontal lines
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


def save_debug(prefix: str, original: np.ndarray, gray: np.ndarray, deskewed: np.ndarray, bin_img: np.ndarray) -> None:
    os.makedirs(os.path.dirname(prefix), exist_ok=True)
    cv2.imwrite(prefix + "_orig.jpg", original)
    cv2.imwrite(prefix + "_gray.jpg", gray)
    cv2.imwrite(prefix + "_deskew.jpg", deskewed)
    cv2.imwrite(prefix + "_bin.jpg", bin_img)
import cv2
import numpy as np
from skimage.filters import threshold_sauvola
from skimage.transform import rotate
from typing import Tuple, Dict

# Contract
# - input: image path
# - output: dict of processed image arrays (grayscale, binarized, deskewed, upscaled)

def load_image(path: str) -> np.ndarray:
    img = cv2.imdecode(np.fromfile(path, dtype=np.uint8), cv2.IMREAD_COLOR)
    if img is None:
        raise FileNotFoundError(path)
    return img


def to_grayscale(img: np.ndarray) -> np.ndarray:
    return cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)


def upscale(img: np.ndarray, min_side: int = 1200) -> np.ndarray:
    h, w = img.shape[:2]
    scale = max(1.0, min_side / min(h, w))
    if scale == 1.0:
        return img
    return cv2.resize(img, None, fx=scale, fy=scale, interpolation=cv2.INTER_CUBIC)


def binarize_sauvola(gray: np.ndarray) -> np.ndarray:
    win = max(25, (min(gray.shape) // 30) | 1)  # odd window size
    thresh = threshold_sauvola(gray, window_size=win)
    binary = (gray > (thresh * 255)).astype(np.uint8) * 255
    return binary


def deskew(gray: np.ndarray, delta: int = 1, limit: int = 5) -> Tuple[np.ndarray, float]:
    # Estimate skew angle via Hough transform on edges
    edges = cv2.Canny(gray, 50, 150)
    lines = cv2.HoughLines(edges, 1, np.pi / 180, threshold=200)
    angles = []
    if lines is not None:
        for rho_theta in lines[:200]:
            rho, theta = rho_theta[0]
            angle = (theta * 180 / np.pi) - 90
            if -limit <= angle <= limit:
                angles.append(angle)
    angle = float(np.median(angles)) if angles else 0.0
    # rotate about center
    (h, w) = gray.shape[:2]
    M = cv2.getRotationMatrix2D((w / 2, h / 2), angle, 1.0)
    rotated = cv2.warpAffine(gray, M, (w, h), flags=cv2.INTER_CUBIC, borderMode=cv2.BORDER_REPLICATE)
    return rotated, angle


def preprocess(path: str) -> Dict[str, np.ndarray]:
    img = load_image(path)
    img = upscale(img)
    gray = to_grayscale(img)
    desk, angle = deskew(gray)
    binary = binarize_sauvola(desk)
    return {
        "original": img,
        "gray": gray,
        "deskewed": desk,
        "binary": binary,
        "skew_angle": angle,
    }


def save_debug(images: Dict[str, np.ndarray], outdir: str) -> None:
    outdir = outdir.rstrip("/")
    for k, v in images.items():
        if isinstance(v, np.ndarray):
            cv2.imwrite(f"{outdir}/{k}.png", v)

if __name__ == "__main__":
    import argparse, os
    p = argparse.ArgumentParser()
    p.add_argument("--image", required=True)
    p.add_argument("--out", default="outputs/preprocessed")
    args = p.parse_args()
    os.makedirs(args.out, exist_ok=True)
    imgs = preprocess(args.image)
    save_debug(imgs, args.out)
    print({k: v.shape if hasattr(v, 'shape') else v for k,v in imgs.items()})
