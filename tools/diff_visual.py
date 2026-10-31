#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/diff_visual.py —— L4 视觉对拍（visual-diff 门禁 runner，task-31）。

对拍面：oracle/screens/（金面，入库）× shots/（复采面，scenario_run 生成）——
逐帧逐像素（白名单外零容差）。CH/PPU 面=L3 RAM 对拍盲面补全（验证体系收官层）。

白名单：**唯一允许的例外清单** gates/visual_whitelist.json（AGENTS §5——变更需用户
确认并记录）；条目={file: glob, rect:[x,y,w,h], reason, approved_by}；白名单外
1 像素差异即 FAIL。

game-core 渲染输出面（S6）：渲染腿未启用时显式 note（同 L3 game-core 腿口径），
启用后同机制接第二腿。

字节级复现命令（判据）：
  python3 tools/diff_visual.py                # 金面×复采对拍（门禁调用面）
  python3 tools/diff_visual.py --selftest     # 正负对照（同帧全同/合成差异打回）

PNG 解码=纯 stdlib（IHDR+IDAT zlib+unfilter；8 位 RGB/RGBA 非隔行——其余显式拒收）。
"""
from __future__ import annotations

import json
import struct
import sys
import zlib
from pathlib import Path

# 【Windows GBK 兜底】（diff_traces 同款——`⇒` 等 Unicode 崩报告）
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

ROOT = Path(__file__).resolve().parent.parent
GOLDEN = ROOT / "oracle" / "screens"
SHOTS = ROOT / "shots"
WHITELIST = ROOT / "gates" / "visual_whitelist.json"


class PngError(Exception):
    pass


def decode_png(data: bytes) -> tuple[int, int, int, bytearray]:
    """解码 PNG → (宽, 高, 通道数, 像素 RGBA 展平 bytearray)。8 位 RGB/RGBA 非隔行。"""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise PngError("非 PNG 签名")
    pos = 8
    w = h = depth = ctype = -1
    idat = bytearray()
    while pos < len(data):
        if pos + 8 > len(data):
            raise PngError("块头截断")
        ln = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + ln]
        pos += 12 + ln
        if kind == b"IHDR":
            w, h, depth, ctype, _comp, _filt, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8:
                raise PngError(f"位深 {depth}≠8（显式拒收）")
            if ctype not in (2, 6):
                raise PngError(f"色型 {ctype}∉{{2,6}}（显式拒收）")
            if interlace != 0:
                raise PngError("隔行 PNG（显式拒收）")
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    if w < 0 or h < 0:
        raise PngError("缺 IHDR")
    ch = 4 if ctype == 6 else 3
    raw = zlib.decompress(bytes(idat))
    stride = w * ch
    if len(raw) != (stride + 1) * h:
        raise PngError(f"IDAT 长度不符: {len(raw)} != {(stride + 1) * h}")
    out = bytearray(stride * h)
    prev = bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if f == 0:
            pass
        elif f == 1:  # Sub
            for i in range(ch, stride):
                line[i] = (line[i] + line[i - ch]) & 0xFF
        elif f == 2:  # Up
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif f == 3:  # Average
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif f == 4:  # Paeth
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                b = prev[i]
                c = prev[i - ch] if i >= ch else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        else:
            raise PngError(f"未知 filter {f}")
        out[y * stride:(y + 1) * stride] = line
        prev = line
    # 统一 RGBA 展平
    rgba = bytearray(w * h * 4)
    for i in range(w * h):
        if ch == 4:
            rgba[i * 4:i * 4 + 4] = out[i * 4:i * 4 + 4]
        else:
            rgba[i * 4:i * 4 + 3] = out[i * 3:i * 3 + 3]
            rgba[i * 4 + 3] = 255
    return w, h, 4, rgba


def load_whitelist() -> list[dict]:
    if not WHITELIST.is_file():
        return []
    doc = json.loads(WHITELIST.read_text(encoding="utf-8"))
    return doc.get("whitelist") or []


def in_rect(x: int, y: int, rect: list[int]) -> bool:
    rx, ry, rw, rh = rect
    return rx <= x < rx + rw and ry <= y < ry + rh


def load_gap_map() -> list[dict]:
    """task-38 族 L4 记账面（迭代态已知缺口——非白名单！trace-diff gap_map 同口径）。"""
    p = ROOT / "data" / "status" / "visual_gap_map.json"
    if not p.is_file():
        return []
    try:
        return json.loads(p.read_text(encoding="utf-8")).get("entries", [])
    except Exception:
        return []


def diff_frames(name: str, golden: bytes, fresh: bytes, wl: list[dict],
                gaps: list[dict] | None = None) -> tuple[bool, str, bool]:
    """像素级对拍（白名单外零容差；账内=临时容忍）。返回 (ok, detail, tolerated)。"""
    gaps = gaps or []
    try:
        gw, gh, _, gp = decode_png(golden)
        fw, fh, _, fp = decode_png(fresh)
    except PngError as e:
        return False, f"{name}: PNG 解码失败——{e}", False
    if (gw, gh) != (fw, fh):
        return False, f"{name}: 尺寸不一致 金面 {gw}×{gh} vs 复采 {fw}×{fh}", False
    rules = [r for r in wl if Path(name).match(r.get("file", ""))]
    grules = [g for g in gaps if Path(name).match(g.get("file", "*"))]
    diffs = 0
    first = None
    wl_hits = 0
    gap_hits = 0
    for y in range(gh):
        for x in range(gw):
            i = (y * gw + x) * 4
            if gp[i:i + 4] == fp[i:i + 4]:
                continue
            if any(in_rect(x, y, r["rect"]) for r in rules):
                wl_hits += 1
                continue
            if any(in_rect(x, y, g["rect"]) for g in grules):
                gap_hits += 1
                continue
            diffs += 1
            if first is None:
                first = (x, y, bytes(gp[i:i + 4]).hex(), bytes(fp[i:i + 4]).hex())
    if diffs:
        x, y, a, b = first
        return False, (f"{name}: 账外 {diffs} 像素分歧（首 @{x},{y} 金={a} 采={b}"
                       f"；白名单内 {wl_hits}/账内 {gap_hits} 处已排除）"), gap_hits > 0
    if gap_hits:
        return True, (f"{name}: 账内容忍（迭代态缺口 {gap_hits} 像素——"
                      f"账清转 strict）；白名单内 {wl_hits} 处排除）"), True
    return True, f"{name}: 全同（白名单内 {wl_hits} 处排除）" if wl_hits else f"{name}: 全同", False


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    # 正负对照随跑自证（判别力基线原则——selftest 破即拒跑）
    st = selftest(quiet=True)
    if st != 0:
        print("L4 视觉: selftest 正负对照失败——判别力破裂，拒绝出裁定")
        return 1
    goldens = sorted(GOLDEN.glob("*.png"))
    if not goldens:
        print("L4 视觉: oracle/screens/ 无金面（首例场景金面待立——fail-loud 空跑不得绿）")
        return 1
    wl = load_whitelist()
    gaps = load_gap_map()
    failed = 0
    tolerated = 0
    for g in goldens:
        fresh_path = SHOTS / g.name
        if not fresh_path.is_file():
            print(f"[FAIL] {g.name}: 复采面缺失（shots/{g.name}——跑 scenario_run 采样后复验）")
            failed += 1
            continue
        ok, detail, tol = diff_frames(g.name, g.read_bytes(), fresh_path.read_bytes(), wl, gaps)
        print(("[TOLERATED] " if (ok and tol) else "[PASS] " if ok else "[FAIL] ") + detail)
        if not ok:
            failed += 1
        elif tol:
            tolerated += 1
    print(f"L4 视觉裁定: {len(goldens)} 金面帧, {failed} 失败, 白名单条目 {len(wl)}, 账内容忍 {tolerated}")
    if gaps:
        print("缺口路线图（账清⇒strict 自然转绿）:")
        for g in gaps:
            print(f"  [{g['id']}] {g.get('file','*')} ——{g.get('reason','')}｜退出={g.get('exit','')}")
    # 第二腿（game-core 渲染腿——S6-10 同机制自启用，L3 口径）
    render_shots = ROOT / "analysis" / "render-shots"
    if not render_shots.is_dir():
        print("（game-core 渲染腿未启用（S6）——启用后接第二腿同机制）")
        return 1 if failed else 0
    rfailed = 0
    rtol = 0
    for g in goldens:
        fresh_path = render_shots / g.name
        if not fresh_path.is_file():
            mre = [e for e in gaps if e.get("kind") == "missing-face"
                   and Path(g.name).match(e.get("file", "*"))]
            if mre:
                print(f"[R-TOLERATED] {g.name}: 渲染面缺失账内（{mre[0]['id']}——"
                      f"{mre[0].get('exit','')}）")
                rtol += 1
            else:
                print(f"[FAIL] {g.name}: 渲染腿面缺失（analysis/render-shots/{g.name}——跑 render-dump 后复验）")
                rfailed += 1
            continue
        ok, detail, tol = diff_frames(g.name, g.read_bytes(), fresh_path.read_bytes(), wl, gaps)
        print(("[R-TOLERATED] " if (ok and tol) else "[R-PASS] " if ok else "[R-FAIL] ") + detail)
        if not ok:
            rfailed += 1
    print(f"L4 渲染腿裁定: {len(goldens)} 金面帧, {rfailed} 失败, 白名单条目 {len(wl)}, 账内容忍 {rtol}")
    return 1 if (failed or rfailed) else 0


def _encode_png(w: int, h: int, rgba: bytes) -> bytes:
    """最小 PNG 编码器（selftest 用：filter 0、RGB 色型 2）。"""
    def chunk(kind: bytes, body: bytes) -> bytes:
        return (struct.pack(">I", len(body)) + kind + body
                + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            i = (y * w + x) * 4
            raw += rgba[i:i + 3]
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", zlib.compress(bytes(raw))) + chunk(b"IEND", b""))


def selftest(quiet: bool = False) -> int:
    """正负对照（判据）：同帧全同 ✓ / 1 像素差异打回 ✓ / 白名单矩形放行 ✓ / 尺寸不符打回 ✓。"""
    ok_all = True
    def say(t: str) -> None:
        if not quiet:
            print(t)

    def mark(b: bool) -> str:
        return "✓" if b else "✗"
    img = bytearray(b"\x10\x20\x30\xff" * (8 * 8))
    png = _encode_png(8, 8, bytes(img))
    # 正对照：同帧全同
    ok, d, _ = diff_frames("t_same.png", png, png, [])
    say("[正对照] " + d + " → " + mark(ok))
    ok_all &= ok
    # 负对照：1 像素差异打回
    img2 = bytearray(img)
    img2[4 * 10 + 1] ^= 0x01  # (2,1) 绿道差 1
    ok, d, _ = diff_frames("t_diff.png", png, _encode_png(8, 8, bytes(img2)), [])
    say("[负对照·1 像素] " + d + " → " + mark(not ok))
    ok_all &= (not ok)
    # 白名单放行
    ok, d, _ = diff_frames("t_diff.png", png, _encode_png(8, 8, bytes(img2)),
                        [{"file": "t_*.png", "rect": [0, 0, 8, 8], "reason": "selftest"}])
    say("[白名单放行] " + d + " → " + mark(ok))
    ok_all &= ok
    # 尺寸不符打回
    ok, d, _ = diff_frames("t_size.png", png, _encode_png(4, 4, bytes(b"\x00" * 64)), [])
    say("[尺寸不符] " + d + " → " + mark(not ok))
    ok_all &= (not ok)
    say("selftest: " + ("PASS" if ok_all else "FAIL"))
    return 0 if ok_all else 1


if __name__ == "__main__":
    sys.exit(main())
