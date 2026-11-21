"""渲染健全性体检的锚测（防"空对空"：diff==0 也可能是两腿都黑）。"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402

from tools.movie_ref import room_sweep as rs  # noqa: E402

STEM = "_tmp_sanity_probe_test"
FRAME, TAG = 4242, "chk"


def _write(img, rust: bool):
    d = rs.SHOTS_RUST if rust else (rs.ROOT / "shots")
    d.mkdir(exist_ok=True)
    Image.fromarray(img.astype(np.uint8)).save(d / f"{STEM}_f{FRAME:05d}_{TAG}.png")


def _cleanup():
    for d in (rs.SHOTS_RUST, rs.ROOT / "shots"):
        p = d / f"{STEM}_f{FRAME:05d}_{TAG}.png"
        if p.exists():
            p.unlink()


def _scene() -> np.ndarray:
    """一块"正常房间"：8 色、非黑像素约一半。"""
    img = np.zeros((240, 256, 3), dtype=np.uint8)
    for i, c in enumerate([(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0),
                           (0, 255, 255), (255, 0, 255), (200, 200, 200)]):
        img[10 + i * 20:30 + i * 20, 20:200] = c
    return img


def test_two_blank_legs_are_flagged():
    _write(np.zeros((240, 256, 3)), rust=False)
    _write(np.zeros((240, 256, 3)), rust=True)
    s = rs.sanity_probe(STEM, FRAME, TAG)
    _cleanup()
    assert s["colors"] <= 2 and s["non_black"] == 0.0
    assert s.get("note") == "suspicious_blank"


def test_shift_explained_pair_is_reported():
    a = _scene()
    _write(a, rust=False)
    _write(np.roll(a, 2, axis=1), rust=True)
    s = rs.sanity_probe(STEM, FRAME, TAG)
    _cleanup()
    assert s["colors"] >= 7
    # 符号约定：dx 是"把 rust 图平移多少能对上 emu"。rust 内容右移 2px ⇒ dx=-2。
    assert abs(s["shift_best"]) == 2 and s["shift_residual"] == 0.0
    assert s["note"] == f"shift_explains_{s['shift_best']}px"


def test_real_content_difference_has_no_note():
    a = _scene()
    b = a.copy()
    b[:] = np.where(b.sum(axis=2, keepdims=True) > 0, b, (90, 90, 90))  # 背景换成灰
    _write(a, rust=False)
    _write(b, rust=True)
    s = rs.sanity_probe(STEM, FRAME, TAG)
    _cleanup()
    assert "note" not in s, s
