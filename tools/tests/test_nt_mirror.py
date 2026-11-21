"""名表镜像归一化的锚测（伪 `nametable` 层的防线）。

两腿 dump 的 CIRAM 布局可能不同（emu 水平镜像 / rust 竖直镜像）。旧形按页号直比
⇒ 同一物理页的不同镜像位被判成名表差。本测用合成 dump 钉死归一化行为。
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.movie_ref.room_sweep import _nt_diff_normalized  # noqa: E402

HEAD = 0x2000
TOTAL = 0x3023


def mk(phys0: bytes, phys1: bytes, layout: str) -> bytes:
    """按布局拼一张 dump：horizontal ⇒ [p0,p1,p0,p1]? 不——按 _mirror_pages 的定义。"""
    buf = bytearray(TOTAL)
    if layout == "horizontal":      # p0==p1 且 p2==p3 ⇒ 物理页取 (p0, p2)
        pages = [phys0, phys0, phys1, phys1]
    else:                            # vertical：p0==p2 且 p1==p3 ⇒ 物理页取 (p0, p1)
        pages = [phys0, phys1, phys0, phys1]
    for i, pg in enumerate(pages):
        buf[HEAD + i * 0x400: HEAD + (i + 1) * 0x400] = pg
    return bytes(buf)


def test_same_physical_pages_different_layout_are_equal():
    p0 = bytes([0x11]) * 0x400
    p1 = bytes([0x22]) * 0x400
    emu = mk(p0, p1, "horizontal")
    rust = mk(p0, p1, "vertical")
    n, layouts, trusted = _nt_diff_normalized(emu, rust)
    assert trusted, layouts
    assert n == 0, f"同一物理页、不同镜像布局必须判 0 差（旧形会报 1024）"


def test_real_page_difference_is_counted():
    p0 = bytes([0x11]) * 0x400
    emu = mk(p0, bytes([0x22]) * 0x400, "horizontal")
    rust = mk(p0, bytes([0x33]) * 0x400, "vertical")
    n, layouts, trusted = _nt_diff_normalized(emu, rust)
    assert trusted and n == 0x400, (n, layouts)


def test_unknown_layout_is_marked_untrusted():
    # 两腿各自四页互不相同 ⇒ 镜像布局判不出，必须标 untrusted（不许假装比过）
    def four(vals):
        buf = bytearray(TOTAL)
        for i, v in enumerate(vals):
            buf[HEAD + i * 0x400: HEAD + (i + 1) * 0x400] = bytes([v]) * 0x400
        return bytes(buf)
    emu = four([0x01, 0x02, 0x03, 0x04])
    rust = four([0x05, 0x06, 0x07, 0x08])
    n, layouts, trusted = _nt_diff_normalized(emu, rust)
    assert not trusted, layouts
    assert n == 0x1000
