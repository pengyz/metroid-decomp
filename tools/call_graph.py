#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/call_graph.py —— task-12 执行路径调用图（Mesen2 动态 × 静态 xref 双工具交叉）。

产出：按执行序排序的端口队列（bank:addr + 调用深度 + 所属域）——喂 task-10/11
「每引爆一个端一个」节奏。

双工具口径：
  动态（A 级）= callbackType.exec 真指令入口回调（scenario_capture.lua entry_probe）
    逐入口记执行序/首帧/调用者边（栈 ret-2 反推，site 首字节 0x20=JSR 边）；
  静态（A 级）= seed 代码项 ROM 字节 xref（JSR/JMP 目标）+ decode_sweep 岛内流调用点。
  交叉：动态边 ⊆ 静态边 对账（missed=静态未见边，如实登记）；静态边未命中=路径受限。

子命令：
  anchors --out analysis/_gen/anchors.json [--meta anchors_meta.json]
      生成函数级入口锚点清单（seed sub_ 标签 + decode_sweep 有佐证岛入口 + 根入口）。
  merge --run <prefix> [--run <prefix> ...] [--phase boot,move] --static <sweep.json>
      [--anchor-meta <meta.json>] [--repeat <prefixA> <prefixB>]
      合并多场景执行序 → 端口队列（stdout）；--repeat 做换时刻双跑对照。

复跑链（可复现）：
  python3 tools/decode_sweep.py --json /tmp/sweep.json
  python3 tools/call_graph.py anchors --out analysis/_gen/anchors.json
  python3 tools/scenario_run.py run execpath-boot    # 场景含 entry_probe 行
  python3 tools/scenario_run.py run execpath-move
  python3 tools/call_graph.py merge --run analysis/traces/execpath-boot \\
      --run analysis/traces/execpath-move --phase boot,move --static /tmp/sweep.json
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEED_PATH = ROOT / "reference" / "seed" / "metroid.asm"


def _load_decode_sweep():
    spec = importlib.util.spec_from_file_location(
        "decode_sweep", ROOT / "tools" / "decode_sweep.py")
    ds = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ds)
    return ds


def cmd_anchors(args) -> int:
    ds = _load_decode_sweep()
    banks = ds.load_prg()
    islands, code_items, labels = ds.parse_seed(banks)
    # 函数级锚点：sub_ 标签（IDA 机械函数入口）+ 有佐证岛入口 + 根入口
    subs: set[int] = set()
    for line in SEED_PATH.read_text(encoding="utf-8").splitlines():
        for m in re.finditer(r"\bsub_([0-9A-F]{4})\b", line):
            subs.add(int(m.group(1), 16))
    island_entries: set[int] = set()
    if args.static and Path(args.static).is_file():
        st = json.loads(Path(args.static).read_text(encoding="utf-8"))
        for isl in st.get("islands", []):
            for e in isl.get("entries", []):
                if e.get("justified"):
                    island_entries.add(e["entry"])
    roots = {0xC0D9, 0xFFB0, 0xFFB0}
    anchors = sorted(subs | island_entries | roots)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps([f"{a:04x}" for a in anchors]) + "\n", encoding="utf-8")
    meta = {
        "roots": sorted(f"{a:04x}" for a in roots),
        "sub_labels": sorted(f"{a:04x}" for a in subs),
        "island_entries": sorted(f"{a:04x}" for a in island_entries),
    }
    mp = Path(args.meta) if args.meta else out.with_suffix(".meta.json")
    mp.write_text(json.dumps(meta, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"[call_graph] anchors {len(anchors)}（sub_ {len(subs)} + 岛入口 "
          f"{len(island_entries)} + 根 {len(roots)}）→ {out}")
    return 0


def load_run(prefix: str) -> dict:
    p = Path(prefix)
    def rd(tag):
        f = Path(f"{prefix}.{tag}.jsonl")
        rows = []
        if f.is_file():
            for ln in f.read_text(encoding="utf-8").splitlines():
                try:
                    rows.append(json.loads(ln))
                except json.JSONDecodeError:
                    print(f"[warn] {f.name}: 坏行跳过")
        return rows
    return {"prefix": prefix, "entries": rd("entries"), "edges": rd("edges")}


def static_edges(ds, banks, islands, code_items, labels) -> set[tuple[int, int, int]]:
    """静态 xref 边（A 级）：seed 代码项 JSR/JMP + 分发站点表词。返回 (bank, site, tgt)。"""
    edges: set[tuple[int, int, int]] = set()
    for k in range(N_BANKS := 8):
        bk = banks[k]
        for a, n in code_items[k]:
            hit = ds.OPS.get(bk.byte(a))
            if not hit:
                continue
            mn, mode, _ln = hit
            if mn in ("JSR", "JMP") and mode == "abs":
                edges.add((k, a, bk.word(a + 1)))
    return edges


def cmd_merge(args) -> int:
    ds = _load_decode_sweep()
    banks = ds.load_prg()
    islands, code_items, labels = ds.parse_seed(banks)
    stat = static_edges(ds, banks, islands, code_items, labels)
    stat_pairs = {(s, t) for _k, s, t in stat}

    runs = []
    for i, pref in enumerate(args.run):
        r = load_run(pref)
        r["phase"] = args.phase.split(",")[i] if args.phase and i < len(args.phase.split(",")) else f"run{i}"
        runs.append(r)

    # jsr 边 ROM 字节核（假 caller 边剔除：顺流/JMP 入口的栈帧残留会产生伪 jsr 边）：
    # site 处必须是 `20 lo hi` 且 word(site+1)==callee 才算真 JSR 边。
    def edge_valid(caller: int, callee: int) -> bool:
        if not (0x8000 <= caller <= 0xFFFC):
            return False
        k = 7 if caller >= 0xC000 else None
        if k == 7:
            off = 0x1C000 + (caller - 0xC000)
        else:
            return None  # 窗口地址 bank 未知——按并集核（任一 bank0-6 同址成立即可）
        prg = banks[k].data
        o = caller - banks[k].base
        return prg[o] == 0x20 and (prg[o + 1] | (prg[o + 2] << 8)) == callee

    def edge_valid_win(caller: int, callee: int) -> bool:
        for k in range(7):
            prg = banks[k].data
            o = caller - 0x8000
            if 0 <= o < 0x3FFD and prg[o] == 0x20 and (prg[o + 1] | (prg[o + 2] << 8)) == callee:
                return True
        return False

    # 端口队列：每入口一行，按 (phase 序, first_seq) 执行序排
    nodes: dict[int, dict] = {}
    edges_all: dict[tuple[int, int, str], dict] = {}
    for r in runs:
        for e in r["entries"]:
            if "a" not in e:
                continue
            a = int(e["a"], 16)
            nd = nodes.setdefault(a, {"runs": {}, "n": 0})
            nd["runs"][r["phase"]] = {"first_seq": e["first_seq"],
                                      "first_frame": e["first_frame"], "n": e["n"]}
            nd["n"] += e["n"]
        for g in r["edges"]:
            c, t = int(g["caller"], 16), int(g["callee"], 16)
            kind = g["kind"]
            if kind == "jsr":
                v = edge_valid(c, t)
                if v is None:
                    v = edge_valid_win(c, t)
                if not v:
                    kind = "unverified"  # 伪 jsr 边（栈帧残留）——剔除出调用图
            key = (c, t, kind)
            ge = edges_all.setdefault(key, {"n": 0, "first_seq": 1 << 30})
            ge["n"] += g["n"]
            ge["first_seq"] = min(ge["first_seq"], g["first_seq"])

    # 调用深度：函数级静态图 BFS（B 级推导，A 级边为据；根=C0D9 NMI / FFB0 RESET）。
    # site→所属函数入口：优先 IDA funcs 跨度（analysis/ida/bank*.json），回退最近前驱
    # 锚点（启发式——共享尾会碎片化，仅兜底）；跳转表边同映射。
    import bisect
    ida_funcs: list[tuple[int, int]] = []
    for kb in range(8):
        fp = ROOT / "analysis" / "ida" / f"bank{kb:02d}.json"
        if fp.is_file():
            try:
                fj = json.loads(fp.read_text(encoding="utf-8"))
                base = 0xC000 if kb == 7 else 0x8000
                for f in fj.get("funcs", []):
                    st_, en_ = f["start"], f["end"]
                    if st_ < 0x4000:
                        st_, en_ = base + st_, base + en_
                    ida_funcs.append((st_, en_))
            except (OSError, json.JSONDecodeError):
                pass
    ida_funcs.sort()
    func_starts = [s for s, _e in ida_funcs]
    anchors_all = sorted(set(nodes) | _SUB_CACHE | {0xC0D9, 0xFFB0})
    win_anchors = sorted(a for a in anchors_all if a < 0xC000)
    fix_anchors = sorted(a for a in anchors_all if a >= 0xC000)

    def owner_entry(site: int) -> int | None:
        i = bisect.bisect_right(func_starts, site)
        if i:
            s, e = ida_funcs[i - 1]
            if s <= site < e:
                return s
        lst = fix_anchors if site >= 0xC000 else win_anchors
        j = bisect.bisect_right(lst, site)
        return lst[j - 1] if j else None

    def add_edge(adj, s: int, t: int):
        so = owner_entry(s)
        if so is not None:
            adj.setdefault(so, []).append(t)

    adj: dict[int, list[int]] = {}
    # 顺流边（函数体 A.end == B.start ⇒ A 顺流进 B——FFB0→C01A→C0BC 族断链根因）
    for i in range(len(ida_funcs) - 1):
        s1, e1 = ida_funcs[i]
        s2, _e2 = ida_funcs[i + 1]
        if e1 == s2:
            adj.setdefault(s1, []).append(s2)
    for _k, s, t in stat:
        add_edge(adj, s, t)
    try:
        stj = json.loads(Path(args.static).read_text(encoding="utf-8"))
        for jt in stj.get("jump_table_entries", []):
            add_edge(adj, jt["site"], jt["target"])
        for cs in stj.get("call_sites", []):  # 岛内流调用点（decode_sweep r2/r3）
            add_edge(adj, cs["addr"], cs["target"])
    except (OSError, json.JSONDecodeError):
        pass
    for (c, t, kind), _g in edges_all.items():
        if kind == "jsr":
            add_edge(adj, c, t)
    depth = {}
    frontier = [0xC0D9, 0xFFB0]
    for r0 in frontier:
        depth[r0] = 0
    while frontier:
        nxt = []
        for u in frontier:
            for v in adj.get(u, []):
                if v not in depth:
                    depth[v] = depth[u] + 1
                    nxt.append(v)
        frontier = nxt

    # 执行序：phase 序 × first_seq
    def order_key(item):
        a, nd = item
        best = None
        for i, r in enumerate(runs):
            if r["phase"] in nd["runs"]:
                cand = (i, nd["runs"][r["phase"]]["first_seq"])
                if best is None or cand < best:
                    best = cand
        return best or (99, 1 << 30)

    queue = sorted(nodes.items(), key=order_key)

    # 双工具交叉：动态边 vs 静态边
    dyn_pairs = {(c, t) for (c, t, kind) in edges_all if kind == "jsr"}
    missed_static = sorted(p for p in dyn_pairs if p not in stat_pairs)
    static_only = sorted(p for p in stat_pairs
                         if p[1] in nodes and p[0] in nodes and p not in dyn_pairs)

    print(f"[call_graph] 运行 {len(runs)} 个场景（{[r['phase'] for r in runs]}）；"
          f"入口 {len(nodes)}，动态边 {len(edges_all)}（jsr {len(dyn_pairs)}），"
          f"静态边 {len(stat_pairs)}")
    print(f"[call_graph] 双工具交叉：动态 jsr 边静态未见 {len(missed_static)}；"
          f"静态边未在动态出现（路径受限）{len(static_only)}")
    if args.repeat:
        r1, r2 = load_run(args.repeat[0]), load_run(args.repeat[1])
        s1 = {int(e["a"], 16) for e in r1["entries"] if "a" in e}
        s2 = {int(e["a"], 16) for e in r2["entries"] if "a" in e}
        print(f"[call_graph] 换时刻双跑：入口集 跑1 {len(s1)} 跑2 {len(s2)} "
              f"仅1 {len(s1 - s2)} 仅2 {len(s2 - s1)}")

    print("\n== 端口队列（执行序；seq|bank:addr|depth|域|首执行|命中|入边）==")
    indeg = {a: 0 for a in nodes}
    for (c, t, _k), _g in edges_all.items():
        if t in indeg:
            indeg[t] += 1
    for seq, (a, nd) in enumerate(queue, 1):
        best = min(nd["runs"].items(), key=lambda kv: kv[1]["first_seq"])
        bank = 7 if a >= 0xC000 else "?"
        kind = "root" if a in (0xC0D9, 0xFFB0) else ("sub" if a in _SUB_CACHE else "entry")
        print(f"  {seq:3d} | bank{bank}:${a:04X} | d={depth.get(a, -1)} | "
              f"{kind}@{best[0]} | f{best[1]['first_frame']} | n={nd['n']} | in={indeg[a]}")

    if missed_static:
        print("\n== 动态边静态未见（登记）==")
        for c, t in missed_static[:20]:
            print(f"  ${c:04X} → ${t:04X}")
    return 0


_SUB_CACHE: set[int] = set()


def main() -> int:
    global _SUB_CACHE
    ap = argparse.ArgumentParser(description="task-12 执行路径调用图")
    sub = ap.add_subparsers(dest="cmd", required=True)
    a1 = sub.add_parser("anchors")
    a1.add_argument("--out", required=True)
    a1.add_argument("--meta", default=None)
    a1.add_argument("--static", default=None, help="decode_sweep --json（岛入口）")
    a2 = sub.add_parser("merge")
    a2.add_argument("--run", action="append", required=True)
    a2.add_argument("--phase", default=None, help="逗号分隔，与 --run 顺序对应")
    a2.add_argument("--static", required=True)
    a2.add_argument("--anchor-meta", default=None)
    a2.add_argument("--repeat", nargs=2, default=None, help="换时刻双跑前缀对")
    args = ap.parse_args()
    am = getattr(args, "anchor_meta", None)
    if am and Path(am).is_file():
        meta = json.loads(Path(am).read_text(encoding="utf-8"))
        _SUB_CACHE = {int(a, 16) for a in meta.get("sub_labels", [])}
    elif args.cmd == "merge":
        mp = ROOT / "analysis" / "_gen" / "anchors.meta.json"
        if mp.is_file():
            meta = json.loads(mp.read_text(encoding="utf-8"))
            _SUB_CACHE = {int(a, 16) for a in meta.get("sub_labels", [])}
    if args.cmd == "anchors":
        return cmd_anchors(args)
    return cmd_merge(args)


if __name__ == "__main__":
    sys.exit(main())
