#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/diff_forensics.py —— 像素分歧离线取证（三轴归因，GGR diff_forensics 移植件）。

移植源：GuardicGaidenRemake tools/diff_forensics.py（72 行零依赖纯 stdlib）。
适配面（与源的差异，本仓纪律：先读格式再动手）：
  · PPU dump 布局=tools/scenario_capture.lua::dumpPpu 产物（.ppu.hex 单行 hex）：
      [0x0000-0x2FFF)  PPU 总线 12KB（mt=9：CHR $0000-$1FFF 8KB + 名表 $2000-$2FFF 4KB）
      [0x3000-0x301F)  palette RAM $3F00-$3F1F（32B）
      [0x3020-0x3022)  scrollX / scrollY / ppuCtrl（getState 面各 1B）
    总长恒 0x3023=12323 字节（dumpPpu 逐帧 flush 纪律产物；f00610 实测 24646 hex char）。
  · 路径/CLI 对齐本仓 tools/ 惯例（argparse 子命令 + --selftest 正负对照，
    diff_visual.py 同族风格）；PNG 解码=纯 stdlib（IHDR+IDAT zlib+unfilter，
    8 位 RGB/RGBA 非隔行——其余显式拒收）。

三轴（继承源语义）：
  ① png  我 vs 金逐像素 diff（PNG 双读）；
  ② ppu  模型写值 vs 渲染读值离线归因（.ppu.hex 双读 → NT/CHR/PAL/相位 分域清单）；
  ③ cell 单格通道枚举（NT-CHR-PAL-ATTR-相位 逐点归因表）。

用法（字节级复现命令）：
  python3 tools/diff_forensics.py ppu   <model.ppu.hex> <ref.ppu.hex>
  python3 tools/diff_forensics.py png   <mine.png> <gold.png> [--ppu <model.ppu.hex>]
  python3 tools/diff_forensics.py cell  <model.ppu.hex> <x> <y>
  python3 tools/diff_forensics.py --selftest

本工具不持有判定权（AGENTS §2：判定逻辑只在 gates/）——产物为归因线索面。
"""
from __future__ import annotations

import argparse
import struct
import sys
import zlib
from pathlib import Path

# .ppu.hex 布局常量（唯一真相=scenario_capture.lua::dumpPpu；改 dump 格式须同步此处）
PPU_HEX_TOTAL = 0x3023           # 12323 字节（12KB 总线 + 32B PAL + 3B 尾）
OFF_CHR = 0x0000                 # CHR-RAM $0000-$1FFF（8KB；Metroid=CHR-RAM 游戏）
OFF_NT = 0x2000                  # 名表 $2000-$2FFF（4KB；竖镜）
OFF_PAL = 0x3000                 # palette RAM 32B
OFF_TAIL = 0x3020                # scrollX/scrollY/ctrl 各 1B
NT_BYTES = 0x1000
PAL_BYTES = 0x20
CELLS_ROW, CELLS_COL = 30, 32    # NES 名表格阵（256×240 px）

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # Windows GBK 兜底
except Exception:
    pass


# ---------------- PNG 解码（纯 stdlib；8 位 RGB/RGBA 非隔行） ----------------

def read_png(path):
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{path}: 非 PNG 签名")
    i, idat, w, h, bd, ct, interlace = 8, b"", None, None, None, None, 0
    while i < len(data):
        ln = struct.unpack(">I", data[i:i + 4])[0]
        typ = data[i + 4:i + 8]
        chunk = data[i + 8:i + 8 + ln]
        if typ == b"IHDR":
            w, h, bd, ct, _, _, interlace = struct.unpack(">IIBBBBB", chunk[:13])
        elif typ == b"IDAT":
            idat += chunk
        i += 12 + ln
    if bd != 8 or interlace != 0 or ct not in (2, 6):
        raise SystemExit(
            f"{path}: 不支持的 PNG 形（bitdepth={bd} colortype={ct} interlace={interlace}"
            f"——仅 8 位 RGB(2)/RGBA(6) 非隔行）")
    raw = zlib.decompress(idat)
    bpp = 3 if ct == 2 else 4
    stride = w * bpp
    if len(raw) != (stride + 1) * h:
        raise SystemExit(f"{path}: IDAT 尺寸与 IHDR 不符")
    out = bytearray(stride * h)
    prev = bytearray(stride)
    pos = 0
    for y in range(h):
        fl = raw[pos]
        pos += 1
        line = bytearray(raw[pos:pos + stride])
        pos += stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if fl == 1:
                line[x] = (line[x] + a) & 255
            elif fl == 2:
                line[x] = (line[x] + b) & 255
            elif fl == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif fl == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return w, h, bpp, bytes(out)


# ---------------- .ppu.hex 装载 ----------------

def load_ppu(path):
    """dumpPpu 单行 hex → dict。布局不符（长度≠0x3023）显式拒收（fail-loud）。"""
    data = bytes.fromhex(Path(path).read_text().strip())
    if len(data) != PPU_HEX_TOTAL:
        raise SystemExit(
            f"{path}: .ppu.hex 长度 {len(data)} ≠ {PPU_HEX_TOTAL}"
            f"（dumpPpu 布局=12KB 总线+32B PAL+3B 滚动尾；格式漂移须先改本文件常量）")
    return {
        "chr": data[OFF_CHR:OFF_NT],
        "nt": data[OFF_NT:OFF_PAL],
        "pal": data[OFF_PAL:OFF_TAIL],
        "sx": data[OFF_TAIL], "sy": data[OFF_TAIL + 1], "ctrl": data[OFF_TAIL + 2],
    }


# ---------------- ③ 单格通道枚举（通道相位逐点归因） ----------------

def channel_attrib(p: dict, cell: int) -> dict:
    """cell（0..959，行主序）→ NT tile / ATTR 象限 / 调色板 / CHR 基址 / 相位。"""
    row, col = cell // CELLS_COL, cell % CELLS_COL
    tile = p["nt"][row * CELLS_COL + col]
    attr_byte = p["nt"][960 + (row // 4) * 8 + (col // 4)]
    q = ((row % 4) // 2) * 2 + ((col % 4) // 2)
    pidx = (attr_byte >> (q * 2)) & 3
    # 相位：细滚动（ctrl bit4/bit5 选 NT 半区；细 x=sx&7 影响像素相位）
    bg_base = 0x1000 if (p["ctrl"] & 0x10) else 0x0000
    nt_select = 1 if (p["ctrl"] & 0x02) else 0
    return {
        "cell": cell, "cell_xy": f"({col * 8},{row * 8})", "tile": f"{tile:02X}",
        "attr_byte": f"{attr_byte:02X}", "quadrant": q, "pal_idx": pidx,
        "pal_color0_3": [f"{p['pal'][pidx * 4 + i]:02X}" for i in range(4)],
        "bg_chr_base": f"{bg_base + tile * 16:04X}",
        "nt_select": nt_select,
        "fine_x_phase": p["sx"] & 7, "scroll": f"({p['sx']},{p['sy']})",
    }


# ---------------- ② .ppu.hex 双读分域归因 ----------------

def _cells_of_nt_change(off: int):
    """NT 差异字节偏移（0..1023，相对 $2000）→ 受影响 cell 清单。"""
    if off < 960:
        return [off]
    aoff = off - 960                       # attr 表偏移 0..63
    arow, acol = aoff // 8, aoff % 8       # attr 字节 → 4×4 格块
    cells = []
    for dr in range(4):
        for dc in range(4):
            cells.append((arow * 4 + dr) * CELLS_COL + (acol * 4 + dc))
    return cells


def ppu_forensics(model_path, ref_path):
    m, r = load_ppu(model_path), load_ppu(ref_path)
    report = {"model": str(model_path), "ref": str(ref_path), "nt": [], "chr": [],
              "pal": [], "phase": [], "counts": {}}
    for i in range(0x2000):
        if m["chr"][i] != r["chr"][i]:
            tile = i // 16
            entry = {"off": f"{i:04X}", "tile": f"{tile:02X}",
                     "plane_row": (i % 16) // 8, "y": (i % 16) % 8,
                     "model": f"{m['chr'][i]:02X}", "ref": f"{r['chr'][i]:02X}"}
            if i < 0x1000:
                entry["plane"] = "bg-bitplane" if tile < 0x100 else "note:bg-low-half"
            report["chr"].append(entry)
    for i in range(NT_BYTES):
        if m["nt"][i] != r["nt"][i]:
            cells = _cells_of_nt_change(i)
            entry = {"nt_off": f"{0x2000 + i:04X}",
                     "model": f"{m['nt'][i]:02X}", "ref": f"{r['nt'][i]:02X}",
                     "cells": cells}
            if i >= 960:
                entry["kind"] = "ATTR"
                # 象限 2bit 差异 → 该 attr 字节内受影响象限清单
                qdiff = [q for q in range(4)
                         if ((m["nt"][i] >> (q * 2)) & 3) != ((r["nt"][i] >> (q * 2)) & 3)]
                entry["quadrants"] = qdiff
            else:
                entry["kind"] = "TILE"
                entry["cell"] = cells[0]
            report["nt"].append(entry)
    pal_names = ["bg", "sprite"]
    for i in range(PAL_BYTES):
        if m["pal"][i] != r["pal"][i]:
            k = i % 16
            # $3F0x/$3F1x 页内：k=0=universal（$3F10 镜 $3F00）；1-3=pal0 色1-3，4-7=pal1…
            sub = "universal" if k == 0 else f"pal{k // 4}"
            report["pal"].append({"pal_off": f"{0x3F00 + i:02X}",
                                  "group": pal_names[i // 16], "sub": sub,
                                  "color": i % 4,
                                  "model": f"{m['pal'][i]:02X}",
                                  "ref": f"{r['pal'][i]:02X}"})
    for name, key in (("scrollX", "sx"), ("scrollY", "sy"), ("ppuCtrl", "ctrl")):
        if m[key] != r[key]:
            report["phase"].append({"field": name, "model": f"{m[key]:02X}",
                                    "ref": f"{r[key]:02X}",
                                    "note": "相位/半区选择面——渲染逐像素相位漂移源"})
    report["counts"] = {"chr": len(report["chr"]), "nt": len(report["nt"]),
                        "pal": len(report["pal"]), "phase": len(report["phase"])}
    return report


# ---------------- ① PNG 双读逐像素 diff（可选 --ppu 归因到格） ----------------

def png_forensics(a_path, b_path, ppu_path=None):
    w1, h1, bpp1, o1 = read_png(a_path)
    w2, h2, bpp2, o2 = read_png(b_path)
    if (w1, h1) != (w2, h2) or bpp1 != bpp2:
        raise SystemExit(f"尺寸/色型不一致: {w1}x{h1}bpp{bpp1} vs {w2}x{h2}bpp{bpp2}")
    p = load_ppu(ppu_path) if ppu_path else None
    diff_px, cells = 0, {}
    for y in range(h1):
        base = y * w1 * bpp1
        for x in range(w1):
            o = base + x * bpp1
            if o1[o:o + 3] != o2[o:o + 3]:
                diff_px += 1
                key = (y // 8) * CELLS_COL + (x // 8)
                cells[key] = cells.get(key, 0) + 1
    out = {"a": str(a_path), "b": str(b_path), "size": f"{w1}x{h1}",
           "diff_pixels": diff_px, "cells_hit": len(cells), "cells": []}
    if p:
        for cell in sorted(cells, key=lambda c: -cells[c]):
            a = channel_attrib(p, cell)
            a["diff_pixels_in_cell"] = cells[cell]
            out["cells"].append(a)
    return out


# ---------------- 正负对照自证（--selftest） ----------------

def _write_png(path, w, h, rgb, bpp=3):
    """filter=0 的最小 PNG 写出（自证夹具用；非通用编码器）。"""
    stride = w * bpp
    raw = b"".join(b"\x00" + bytes(rgb[y * stride:(y + 1) * stride]) for y in range(h))
    def chunk(typ, payload):
        c = typ + payload
        return struct.pack(">I", len(payload)) + c + struct.pack(">I", zlib.crc32(c))
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2 if bpp == 3 else 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw))
    png += chunk(b"IEND", b"")
    Path(path).write_bytes(png)


def _synth_ppu(path, mutate=None):
    data = bytearray(PPU_HEX_TOTAL)
    data[OFF_NT + 0] = 0x41                      # cell0 tile=0x41
    data[OFF_NT + 960] = 0xE4                    # attr(0,0)=0xE4（象限 0 pal_idx=0）
    data[OFF_PAL + 1] = 0x16                     # bg pal0 color1
    data[OFF_TAIL] = 3                           # scrollX=3（fine_x 相位=3）
    if mutate:
        mutate(data)
    Path(path).write_text(data.hex() + "\n")


def selftest() -> int:
    import tempfile
    ok = True
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        # ① PNG 正负对照：合成 A/B（B 在 cell(1,0)（x 8-15,y 0-7）4 像素差异）
        w, h = 32, 16
        base = bytearray(w * h * 3)
        for y in range(h):
            for x in range(w):
                o = (y * w + x) * 3
                base[o:o + 3] = bytes(((x * 7) & 255, (y * 11) & 255, 0x30))
        mut = bytearray(base)
        for y in range(8):
            for x in range(8, 12):
                o = (y * w + x) * 3
                mut[o:o + 3] = bytes((0xFF, 0x00, 0x00))
        _write_png(td / "a.png", w, h, base)
        _write_png(td / "b.png", w, h, mut)
        _write_png(td / "a2.png", w, h, base)
        r_same = png_forensics(td / "a.png", td / "a2.png")
        r_diff = png_forensics(td / "a.png", td / "b.png")
        if r_same["diff_pixels"] != 0:
            print(f"FAIL selftest① 同帧对照 diff={r_same['diff_pixels']}（应为 0）")
            ok = False
        if r_diff["diff_pixels"] != 32:  # 4 列 × 8 行
            print(f"FAIL selftest① 合成差异 diff={r_diff['diff_pixels']}（应为 32）")
            ok = False
        print(f"selftest① PNG 正负对照: same={r_same['diff_pixels']} "
              f"diff={r_diff['diff_pixels']}（期望 0/32）")
        # ② PPU 分域归因正负对照：chr/nt-tile/nt-attr/pal/尾 相位各一处受控差异
        def _mut(d):
            d[OFF_CHR + 0x10] ^= 0xFF            # tile 0x01 位面行
            d[OFF_NT + 1] = 0x42                 # cell1 tile 变
            d[OFF_NT + 960] ^= 0x03              # attr(0,0) 象限 0 pal 变
            d[OFF_PAL + 1] = 0x27                # bg pal0 color1 变
            d[OFF_TAIL] = 5                      # scrollX 3→5（相位）
        _synth_ppu(td / "m.ppu.hex")
        _synth_ppu(td / "r.ppu.hex", _mut)
        rep = ppu_forensics(td / "m.ppu.hex", td / "r.ppu.hex")
        c = rep["counts"]
        if not (c["chr"] == 1 and c["nt"] == 2 and c["pal"] == 1 and c["phase"] == 1):
            print(f"FAIL selftest② 分域计数 {c}（期望 chr=1 nt=2 pal=1 phase=1）")
            ok = False
        nt_tile = next(e for e in rep["nt"] if e["kind"] == "TILE")
        nt_attr = next(e for e in rep["nt"] if e["kind"] == "ATTR")
        if nt_tile["cell"] != 1 or nt_attr["quadrants"] != [0]:
            print(f"FAIL selftest② NT 归因 tile_cell={nt_tile['cell']} "
                  f"attr_quadrants={nt_attr['quadrants']}（期望 1/[0]）")
            ok = False
        print(f"selftest② PPU 分域归因: counts={c} nt_kind="
              f"{[e['kind'] for e in rep['nt']]}")
        # ③ 通道枚举自证：cell0 → tile 0x41/attr 0xE4/象限0/pal_idx0/fine_x=3
        p = load_ppu(td / "m.ppu.hex")
        a = channel_attrib(p, 0)
        if not (a["tile"] == "41" and a["attr_byte"] == "E4" and a["quadrant"] == 0
                and a["pal_idx"] == 0 and a["fine_x_phase"] == 3
                and a["bg_chr_base"] == "0410"):
            print(f"FAIL selftest③ 通道枚举 {a}")
            ok = False
        print(f"selftest③ 通道枚举 cell0: tile={a['tile']} attr={a['attr_byte']} "
              f"pal_idx={a['pal_idx']} fine_x={a['fine_x_phase']} chr={a['bg_chr_base']}")
    print("SELFTEST:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


# ---------------- CLI ----------------

def main() -> int:
    ap = argparse.ArgumentParser(
        prog="tools/diff_forensics.py",
        description="像素分歧离线取证（PNG 双读 / .ppu.hex 分域归因 / 单格通道枚举）")
    ap.add_argument("--selftest", action="store_true", help="正负对照自证")
    sub = ap.add_subparsers(dest="cmd")
    sp = sub.add_parser("ppu", help=".ppu.hex 双读 → NT/CHR/PAL/相位 分域归因")
    sp.add_argument("model")
    sp.add_argument("ref")
    sq = sub.add_parser("png", help="PNG 双读逐像素 diff（--ppu 归因到格）")
    sq.add_argument("a")
    sq.add_argument("b")
    sq.add_argument("--ppu", help="模型侧 .ppu.hex（供通道归因）")
    sc = sub.add_parser("cell", help="单格通道枚举表")
    sc.add_argument("ppu")
    sc.add_argument("x", type=int)
    sc.add_argument("y", type=int)
    args = ap.parse_args()

    if args.selftest:
        return selftest()
    if args.cmd == "ppu":
        rep = ppu_forensics(args.model, args.ref)
        import json
        print(json.dumps(rep, ensure_ascii=False, indent=1))
        return 0
    if args.cmd == "png":
        import json
        print(json.dumps(png_forensics(args.a, args.b, args.ppu),
                         ensure_ascii=False, indent=1))
        return 0
    if args.cmd == "cell":
        import json
        p = load_ppu(args.ppu)
        cell = (args.y // 8) * CELLS_COL + (args.x // 8)
        print(json.dumps(channel_attrib(p, cell), ensure_ascii=False, indent=1))
        return 0
    ap.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
