#!/usr/bin/env python3
"""room_sweep 首轮扫描（Brinstar 47 房）的离线统计复算器。

用途
----
`docs/findings/room-sweep.md` §5/§6 与 `data/status/room_sweep_0.json` 的
`clues` 字段里的数字必须能从这里一次复算出来（B 级证据的机械底座）。
只读归档 dump，不跑 emu/rust，不改任何状态文件。

输入（全部是首轮扫描的既有产物，不入库再生）
----
- `data/status/room_sweep_0.json`：每房 status / layers / captures[].shift /
  ppu_diff / room_load。
- `analysis/traces/_tmp_sweep_a0_r{NN}_f{FFFFF}_{tag}.ppu.hex`：emu 腿 PPU dump。
- `shots_rust/_tmp_sweep_a0_r{NN}_f{FFFFF}_{tag}.ppu.hex`：rust 腿 PPU dump。

口径（本文所有数字的定义，改口径必须同步改这里和文档）
----
1. **名表区域**：dump 前 0x3000 字节 = CHR(0x0000) + 4 个名表(各 0x400)。
   一个名表 = tile 区 0x3C0（32×30）+ 属性表 0x40。
2. **镜像去重**：PPU 垂直镜像下 nt2 与 nt0、nt3 与 nt1 是同一块 VRAM 的
   两个映射地址，统计只取 nt0 + nt1。逐采集点已断言 `nt2==nt0 and nt3==nt1`
   （两腿都成立），否则脚本直接报错退出——避免"唯一字节"被镜像双计。
3. **采集点**：`captures[]` 里 tag ∈ {pre, f60, f120}；除特别注明外只用
   f60/f120（pre 是注入帧本身，两腿同 fixture）。
4. **唯一表字节**：同一房 f60/f120 的差异位置取并集（key=(nt, off)），再跨房
   求和。f60 与 f120 的滚动相位不同，逐点求和会把同一批字节重复计数。
5. **列号**：`col = off % 32`（tile 区 off<0x3C0）。属性表不计列。
6. **属性表差异**：只取 `status == "mismatch"` 的房——`ok` 房的 PPU 名表差异
   落在未显示/屏外区域（像素 0，见文档 §6.5），与"渲染面失配"不是同一总体。
7. **emu 整字节同色**：属性字节四个 2-bit 象限取值全部相同，即 emu ∈
   {0x00,0x55,0xAA,0xFF}。"逐象限不同"= rust 字节四个象限两两不同。

输出
----
分节打印每条统计的「定义 / 实际值 / 是否与文档期望一致」。任一不符合
`EXPECT` 的项打印 `FAIL` 并以退出码 1 结束——数字改了而脚本没跟上，会当场爆。
"""

from __future__ import annotations

import json
import pathlib
import sys
from collections import Counter, defaultdict

ROOT = pathlib.Path(__file__).resolve().parents[2]
STATUS = ROOT / "data" / "status" / "room_sweep_0.json"
TRACES = ROOT / "analysis" / "traces"
SHOTS_RUST = ROOT / "shots_rust"

NT0 = 0x2000
NT1 = 0x2400
NT2 = 0x2800
NT3 = 0x2C00
NT_SIZE = 0x400
TILE_SIZE = 0x3C0

# 文档里已经落地的数字。任何一项对不上都说明文档/脚本/数据三者脱节。
EXPECT = {
    "tile_points_col2": 28,
    "tile_points_col29": 14,
    "tile_points_both": 5,
    "tile_points_only_col2": 23,
    "tile_points_only_col29": 9,
    "tile_unique_bytes": 204,
    "tile_4e_to_ff": 162,
    "tile_col2_bytes": 84,  # off%32 自然口径
    "tile_col29_bytes": 108,
    "tile_col14_bytes": 12,
    "room30_f60_unique_total": 30,
    "room30_f60_tile": 6,
    "room30_f60_attr": 24,
    "attr_unique_bytes_mismatch": 369,
    "attr_emu_uniform": 323,
    "attr_rust_four_distinct": 21,
    "attr_only_points": 19,
    "shift_dx": 2,
    "shift_residual_min_pct": 2.3,
    "shift_residual_max_pct": 7.1,
}

failures: list[str] = []


def check(section: str, name: str, actual, expected, note: str = "") -> None:
    ok = actual == expected
    tag = "OK  " if ok else "FAIL"
    print(f"  [{tag}] {section}.{name}: {actual!r} (expect {expected!r})" + (f"  # {note}" if note else ""))
    if not ok:
        failures.append(f"{section}.{name}: {actual!r} != {expected!r}")


def nt(buf: bytes, base: int) -> bytes:
    return buf[base : base + NT_SIZE]


def read_pair(room: int, frame: int, tag: str) -> tuple[bytes, bytes] | None:
    stem = f"_tmp_sweep_a0_r{room:02d}"
    emu = TRACES / f"{stem}_f{frame:05d}_{tag}.ppu.hex"
    rust = SHOTS_RUST / f"{stem}_f{frame:05d}_{tag}.ppu.hex"
    if not (emu.exists() and rust.exists()):
        return None
    return bytes.fromhex(emu.read_text().strip()), bytes.fromhex(rust.read_text().strip())


def region_diff(a: bytes, b: bytes, base: int, lo: int, hi: int) -> dict[int, tuple[int, int]]:
    out: dict[int, tuple[int, int]] = {}
    for off in range(lo, hi):
        if a[base + off] != b[base + off]:
            out[off] = (a[base + off], b[base + off])
    return out


def main() -> int:
    doc = json.loads(STATUS.read_text())
    rooms = doc["rooms"]
    status = {r["room"]: r["status"] for r in rooms}

    # ---- 0. 采样点与镜像断言 ------------------------------------------------
    pairs: dict[int, list[tuple[str, int, bytes, bytes]]] = defaultdict(list)
    mirror_ok = 0
    for r in rooms:
        for cap in r.get("captures") or []:
            tag = cap["tag"]
            if tag not in ("f60", "f120"):
                continue
            pair = read_pair(r["room"], cap["frame"], tag)
            if pair is None:
                continue
            a, b = pair
            for buf in (a, b):
                if not (nt(buf, NT2) == nt(buf, NT0) and nt(buf, NT3) == nt(buf, NT1)):
                    print(f"镜像断言失败: room {r['room']} {tag}", file=sys.stderr)
                    return 2
            mirror_ok += 1
            pairs[r["room"]].append((tag, cap["frame"], a, b))

    print("== 0. 采样点 ==")
    print(f"  有效 (room,tag) 对: {sum(len(v) for v in pairs.values())}；镜像断言通过 {mirror_ok}")

    # ---- 1. tile 差异：列分布 / 唯一字节 / 值形态 ----------------------------
    tile_union: dict[int, dict[tuple[int, int], tuple[int, int]]] = defaultdict(dict)
    tile_col_bytes: Counter[int] = Counter()
    pair_counter: Counter[tuple[int, int]] = Counter()
    for room, caps in pairs.items():
        # 按房取 f60/f120 并集
        u: dict[tuple[int, int], tuple[int, int]] = {}
        for tag, _frame, a, b in caps:
            for ni, base in ((0, NT0), (1, NT1)):
                for off, pv in region_diff(a, b, base, 0, TILE_SIZE).items():
                    u.setdefault((ni, off), pv)
        tile_union[room] = u
        for (ni, off), pv in u.items():
            tile_col_bytes[off % 32] += 1
            pair_counter[pv] += 1

    # 采集点列口径要在「点」级别统计，重新遍历一遍更直观
    pts: list[tuple[int, str, set[int]]] = []
    for room, caps in pairs.items():
        for tag, _frame, a, b in caps:
            cols: set[int] = set()
            for ni, base in ((0, NT0), (1, NT1)):
                cols |= {off % 32 for off in region_diff(a, b, base, 0, TILE_SIZE)}
            if cols:
                pts.append((room, tag, cols))
    with_c2 = sum(1 for _, _, c in pts if 2 in c)
    with_c29 = sum(1 for _, _, c in pts if 29 in c)
    both = sum(1 for _, _, c in pts if {2, 29} <= c)
    only2 = sum(1 for _, _, c in pts if 2 in c and 29 not in c)
    only29 = sum(1 for _, _, c in pts if 29 in c and 2 not in c)

    print("\n== 1. tile 差异（tile 区 0x3C0，整体列 = off%32）==")
    print(f"  含 tile 差的采集点: {len(pts)}")
    check("tile", "有 col2 的采集点", with_c2, EXPECT["tile_points_col2"])
    check("tile", "有 col29 的采集点", with_c29, EXPECT["tile_points_col29"])
    check("tile", "col2∩col29 采集点", both, EXPECT["tile_points_both"])
    check("tile", "仅 col2 采集点", only2, EXPECT["tile_points_only_col2"])
    check("tile", "仅 col29 采集点", only29, EXPECT["tile_points_only_col29"])
    unique_tile = sum(len(u) for u in tile_union.values())
    check("tile", "唯一表 tile 差字节（房内并集后跨房求和）", unique_tile, EXPECT["tile_unique_bytes"])
    check("tile", "4E→FF 对", pair_counter[(0x4E, 0xFF)], EXPECT["tile_4e_to_ff"])
    check("tile", "col2 字节", tile_col_bytes[2], EXPECT["tile_col2_bytes"])
    check("tile", "col29 字节", tile_col_bytes[29], EXPECT["tile_col29_bytes"])
    check("tile", "col14 字节", tile_col_bytes[14], EXPECT["tile_col14_bytes"])
    other_cols = {c: n for c, n in tile_col_bytes.items() if c not in (2, 14, 29)}
    print(f"  其余列: {other_cols or '无'}")
    print("  复核稿曾把 room 37 f120 的整列 30 B（nt1 col29 全 30 行）计为 col2 类；")
    print("  本文按 off%32 自然口径记 col29。若坚持复核 114/78，须在统计器里显式改写归类。")

    # room 30 f60：唯一表 = 6 tile + 24 attr
    key = 30
    c30 = next((caps for room, caps in pairs.items() if room == key), None)
    assert c30, "room 30 缺失"
    u30_tile: set[tuple[int, int]] = set()
    u30_attr: set[tuple[int, int]] = set()
    for tag, _frame, a, b in c30:
        if tag != "f60":
            continue
        for ni, base in ((0, NT0), (1, NT1)):
            u30_tile |= {((ni, off)) for off in region_diff(a, b, base, 0, TILE_SIZE)}
            u30_attr |= {((ni, off)) for off in region_diff(a, b, base, TILE_SIZE, NT_SIZE)}
    check("room30f60", "tile 字节", len(u30_tile), EXPECT["room30_f60_tile"])
    check("room30f60", "attr 字节", len(u30_attr), EXPECT["room30_f60_attr"])
    check("room30f60", "唯一表合计", len(u30_tile) + len(u30_attr), EXPECT["room30_f60_unique_total"])

    # ---- 2. 属性表差异（只取 mismatch 房）----------------------------------
    attr_union: dict[int, dict[tuple[int, int], tuple[int, int]]] = defaultdict(dict)
    for room, caps in pairs.items():
        if status.get(room) != "mismatch":
            continue
        for tag, _frame, a, b in caps:
            for ni, base in ((0, NT0), (1, NT1)):
                for off, pv in region_diff(a, b, base, TILE_SIZE, NT_SIZE).items():
                    attr_union[room].setdefault((ni, off), pv)
    attr_vals = [pv for u in attr_union.values() for pv in u.values()]

    def quads(v: int) -> tuple[int, int, int, int]:
        return ((v >> 0) & 3, (v >> 2) & 3, (v >> 4) & 3, (v >> 6) & 3)

    uniform = sum(1 for x, _y in attr_vals if len(set(quads(x))) == 1)
    rust_four = sum(1 for _x, y in attr_vals if len(set(quads(y))) == 4)
    emu_hist = Counter(x for x, _y in attr_vals)
    print("\n== 2. 属性表差异（唯一表字节，仅 status==mismatch 房）==")
    check("attr", "唯一表 attr 差字节", len(attr_vals), EXPECT["attr_unique_bytes_mismatch"])
    check("attr", "emu 整字节同色（四象限相同）", uniform, EXPECT["attr_emu_uniform"])
    check("attr", "rust 四象限互异", rust_four, EXPECT["attr_rust_four_distinct"])
    print("  emu 值分布: " + ", ".join(f"{v:02X}:{n}" for v, n in sorted(emu_hist.items())))
    print(
        "  emu∈{55,5A,FF}: "
        f"{sum(n for v, n in emu_hist.items() if v in (0x55, 0x5A, 0xFF))}"
        f"；emu∈{{55,AA,FF}}: {sum(n for v, n in emu_hist.items() if v in (0x55, 0xAA, 0xFF))}"
    )
    print(
        "  注：复核稿的『296/349=85%』无法由入库 dump 复算（同口径实际是 "
        f"{uniform}/{len(attr_vals)}={uniform / len(attr_vals):.1%}）；文档采用脚本值。"
    )

    # 只有属性表差、tile 全同的采集点（mismatch 房）
    attr_only = 0
    for room, caps in pairs.items():
        if status.get(room) != "mismatch":
            continue
        for tag, _frame, a, b in caps:
            t = 0
            at = 0
            for ni, base in ((0, NT0), (1, NT1)):
                t += len(region_diff(a, b, base, 0, TILE_SIZE))
                at += len(region_diff(a, b, base, TILE_SIZE, NT_SIZE))
            if t == 0 and at > 0:
                attr_only += 1
    check("attr", "仅属性表差（tile 全同）的采集点", attr_only, EXPECT["attr_only_points"])

    # ---- 3. ok 房里的名表差异（§6.5）--------------------------------------
    ok_nametable = set()
    for room, caps in pairs.items():
        if status.get(room) != "ok":
            continue
        for tag, _frame, a, b in caps:
            t = sum(
                len(region_diff(a, b, base, 0, TILE_SIZE)) for ni, base in ((0, NT0), (1, NT1))
            )
            at = sum(
                len(region_diff(a, b, base, TILE_SIZE, NT_SIZE)) for ni, base in ((0, NT0), (1, NT1))
            )
            if t + at:
                ok_nametable.add(room)
    print(f"\n== 3. ok 房中的名表字节差（像素 0）==\n  房: {sorted(ok_nametable)}（{len(ok_nametable)} 房）")

    # ---- 4. 层分布 ---------------------------------------------------------
    layers: Counter[str] = Counter()
    for r in rooms:
        for tag in ("f60", "f120"):
            layers[r.get("layers", {}).get(tag, "missing")] += 1
    print("\n== 4. 层分布（47 房 × f60/f120 = 94 点）==")
    print("  " + ", ".join(f"{k}:{v}" for k, v in sorted(layers.items())))
    expected_layers = {"nametable": 61, "scroll_drift": 10, "sprite": 6, "clean": 15, "unknown": 2, "missing": 0}
    print(f"  期望: {expected_layers}")

    # ---- 5. scroll_drift 平移残差 -----------------------------------------
    ratios = []
    for r in rooms:
        for cap in r.get("captures") or []:
            sh = cap.get("shift")
            # 只取 scroll_drift 层：其它层的 shift 也常"可解释"，但那是
            # nametable/sprite 层叠加平移后的残差，不属于"滚动时序面"总体。
            if not sh or not sh.get("explains") or cap.get("layer") != "scroll_drift":
                continue
            ratios.append((sh["residual_px"] / sh["raw_px"] * 100.0, r["room"], cap["tag"], sh["dx"], sh["dy"]))
    ratios.sort()
    print("\n== 5. scroll_drift 平移残差（dx/dy 与 residual/raw）==")
    print(f"  点数 {len(ratios)}；dx/dy 全为 {set((dx, dy) for _p, _r, _t, dx, dy in ratios)}")
    print(f"  区间 {ratios[0][0]:.2f}% – {ratios[-1][0]:.2f}%")
    for pct, room, tag, dx, dy in ratios:
        print(f"    room {room:02d} {tag:4s} dx={dx} dy={dy} 残差 {pct:.2f}%")
    check("shift", "最小残差四舍五入到 0.1%", round(ratios[0][0], 1), EXPECT["shift_residual_min_pct"])
    check("shift", "最大残差四舍五入到 0.1%", round(ratios[-1][0], 1), EXPECT["shift_residual_max_pct"])

    # ---- 6. 汇总导出（供 JSON clues / 文档引用）---------------------------
    clues = {
        "source": "tools/movie_ref/room_sweep_stats.py",
        "tile_points_total": len(pts),
        "tile_points_col2": with_c2,
        "tile_points_col29": with_c29,
        "tile_points_both": both,
        "tile_points_only_col2": only2,
        "tile_points_only_col29": only29,
        "tile_unique_bytes": unique_tile,
        "tile_pair_4e_to_ff": pair_counter[(0x4E, 0xFF)],
        "tile_bytes_by_col": {str(c): n for c, n in sorted(tile_col_bytes.items())},
        "attr_unique_bytes_mismatch_rooms": len(attr_vals),
        "attr_emu_uniform": uniform,
        "attr_emu_hist": {f"{v:02X}": n for v, n in sorted(emu_hist.items())},
        "attr_rust_four_distinct": rust_four,
        "attr_only_points": attr_only,
        "ok_rooms_with_nametable_bytes": sorted(ok_nametable),
        "layers": dict(sorted(layers.items())),
        "shift_dx": sorted({dx for _p, _r, _t, dx, _dy in ratios}),
        "shift_residual_pct_min": round(ratios[0][0], 2),
        "shift_residual_pct_max": round(ratios[-1][0], 2),
        "review_unreproducible": {
            "attr_296_of_349": "复核稿数字；入库 dump 同口径为 "
            f"{uniform}/{len(attr_vals)}；review 分母差 20 B，未给出可复算脚本",
            "tile_col2_114_col29_78": "复核稿把 room 37 f120 整列 30 B 归 col2；"
            "off%32 自然口径为 col2 84 / col29 108",
        },
    }
    # 只打印、不落文件：clues 的去向是 `data/status/room_sweep_0.json` 的 `clues` 字段，
    # 工具不应在 data/status/ 下多写一个未经 `--out` 声明的文件（也不便于人工 review）。
    print("\n== 6. clues（嵌入 data/status/room_sweep_0.json 的 `clues` 字段）==")
    print(json.dumps(clues, ensure_ascii=False, indent=2))

    if failures:
        print("\n结果: FAIL", file=sys.stderr)
        for f in failures:
            print("  - " + f, file=sys.stderr)
        return 1
    print("\n结果: 全部与文档期望一致")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
