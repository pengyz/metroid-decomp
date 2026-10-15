#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/decode_sweep_dyn.py —— decode-sweep r3 动态证据合并器（场景化 trace → 定谳素材）。

输入（全部为 tools/scenario_run.py 场景产物 + decode_sweep 静态 JSON）：
  analysis/traces/<name>.dispatch_summary.jsonl  站点签名分发聚合（site/a/tgt/n/wins）
  analysis/traces/<name>.exec_cov.jsonl          待查段指令入口集（r3 起枚举注册=真执行，见下）
  analysis/traces/<name>.window.jsonl            $8000 窗口指纹时间线（A9 证据面）
  <decode_sweep --json>                          静态 verdict/未覆盖段/消费者

语义红线（probe_dispatch_diag v1/v3 A 级实证）：
  - Mesen2 内存回调内 cpu.pc = 回调地址回显（非真 PC）；"execute" 回调≈总线读语义；
  - exec_cov = **访问并集**（fetch+操作数+数据读）：未访问=弱否定（时点/路径受限），
    已访问≠执行。执行级真值需 CDL/trace logger（r2 F1 待研项）。
  - 窗口指纹 = $8000-$800F 16 字节，离线与各 PRG bank 首 16 字节比对自证映射（A9）。

用法：
  python3 tools/decode_sweep_dyn.py --traces analysis/traces/<name> \\
      --static /tmp/decode_sweep_run2.json [--traces2 <第二跑>]

输出：stdout 报告素材（A9 判定 / A 值域 / 7 格动态证据 / 双跑一致性）；
  否定性结论均带「换时刻」双跑对照（gotcha-negative-assertion-time-race）。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM_PATH = ROOT / "rom" / "Metroid (USA).nes"


def load_prg() -> dict[int, bytes]:
    rom = ROM_PATH.read_bytes()
    assert hashlib.sha256(rom).hexdigest() == \
        "e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6", "rom sha 失守"
    prg = rom[0x10:0x20010]
    return {k: prg[k * 0x4000:(k + 1) * 0x4000] for k in range(8)}


def window_bank_map(prg: dict[int, bytes]) -> dict[str, list[int]]:
    """$8000 窗口指纹（bank 首 16 字节）→ 候选 bank 列表（歧义如实保留）。"""
    m: dict[str, list[int]] = {}
    for k in range(8):
        fp = prg[k][:16].hex().upper()
        m.setdefault(fp, []).append(k)
    return m


def load_run(prefix: Path) -> dict:
    out: dict = {"prefix": str(prefix)}
    if not any(Path(f"{prefix}.{t}.jsonl").is_file()
               for t in ("dispatch_summary", "exec_cov", "window")):
        # P7-F2 fail-loud：错误/不存在的 traces 前缀静默按 0=「空跑绿」族
        #（gotcha-negative-assertion-time-race 近亲——空数据与真 0 不可区分）
        raise SystemExit(f"FATAL: traces 前缀无任何产物文件: {prefix}.*.jsonl")
    for tag in ("dispatch_summary", "exec_cov", "window"):
        p = Path(f"{prefix}.{tag}.jsonl")
        rows = []
        if p.is_file():
            for ln_no, x in enumerate(p.read_text().splitlines(), 1):
                try:
                    rows.append(json.loads(x))
                except json.JSONDecodeError:
                    print(f"[warn] {p.name}:{ln_no} 截断/坏行跳过（run 未完成痕迹）")
        out[tag] = rows
    return out


def dispatch_view(run: dict, wmap: dict[str, list[int]]) -> dict:
    """聚合 → per-site A 值域 + 窗口 bank 判定（sig=jsr 为签名自校验通过的真入口）。"""
    sites: dict[str, dict] = {}
    for e in run["dispatch_summary"]:
        if e.get("sig", "jsr") != "jsr":
            continue
        s = e["site"]
        d = sites.setdefault(s, {"indices": {}, "wins": set()})
        d["indices"][e["a"]] = {"tgt": e["tgt"], "n": e["n"], "first": e["first"], "last": e["last"]}
        for w in e.get("wins", []):
            d["wins"].add(w)
    for s, d in sites.items():
        d["win_banks"] = {}
        for w in d["wins"]:
            d["win_banks"][w] = wmap.get(w.upper(), ["?未匹配"])
        d["wins"] = sorted(d["wins"])
    return sites


def window_view(run: dict, wmap: dict[str, list[int]]) -> dict:
    per: dict[str, int] = {}
    for e in run["window"]:
        per[e["win"]] = per.get(e["win"], 0) + 1
    return {w: {"frames": n, "banks": wmap.get(w.upper(), ["?未匹配"])} for w, n in per.items()}


def access_view(run: dict) -> set[int]:
    addrs = set()
    for e in run["exec_cov"]:
        if "a" in e:
            addrs.add(int(e["a"], 16))
    return addrs


def main() -> int:
    ap = argparse.ArgumentParser(description="decode-sweep r3 动态证据合并")
    ap.add_argument("--traces", required=True, help="analysis/traces/<name>（前缀）")
    ap.add_argument("--traces2", default=None, help="换时刻第二跑前缀")
    ap.add_argument("--static", required=True, help="decode_sweep --json 输出")
    args = ap.parse_args()

    prg = load_prg()
    wmap = window_bank_map(prg)
    run = load_run(Path(args.traces))
    static = json.loads(Path(args.static).read_text(encoding="utf-8"))

    print(f"[decode-sweep-dyn] traces={args.traces}")
    print(f"[decode-sweep-dyn] dispatch: {len(run['dispatch_summary'])} 三元组, "
          f"exec_cov {len(access_view(run))} 地址, window 样本 {len(run['window'])} 帧")

    print("\n== A9 窗口态判定（$8000 窗口指纹 → bank 自证）==")
    for w, info in sorted(window_view(run, wmap).items(), key=lambda kv: -kv[1]["frames"]):
        print(f"  {w[:16]}…×{info['frames']}帧 → banks={info['banks']}")

    print("\n== 分发站点 A 值域（签名验证通过的真入口）==")
    sites = dispatch_view(run, wmap)
    if not sites:
        print("  （0 真分发——见 raw_hits 与签名语义说明）")
    for s, d in sorted(sites.items()):
        idxs = sorted(d["indices"].items(), key=lambda kv: kv[0])
        idx_str = " ".join(f"A={a}(tgt=${v['tgt']},n={v['n']})" for a, v in idxs)
        banks = sorted({b for bl in d["win_banks"].values() for b in bl})
        print(f"  site ${s}: {len(idxs)} 个 A 值 [{idx_str}] 窗口banks={banks}")

    print("\n== 待查段动态执行证据（enum=执行 A 级；0 命中=路径受限弱否定）==")
    acc = access_view(run)
    for isl in static["islands"]:
        if isl["bank"] != 7 or isl["verdict"] == "code":
            continue
        for s, e in isl["uncovered_ranges"]:
            hit = sorted(a for a in acc if s <= a < e)
            tag = f"执行 {len(hit)}/{e - s}B（A 级）" if hit else f"0 执行/{e - s}B（路径受限）"
            ex = " ".join(f"${a:04X}" for a in hit[:8])
            print(f"  bank07:${s:04X}-${e - 1:04X}: {tag}" + (f" 例 {ex}" if ex else ""))

    if args.traces2:
        print("\n== 换时刻双跑对照（否定结论三查之「换时刻」）==")
        run2 = load_run(Path(args.traces2))
        s1 = {(e["site"], e["a"], e["tgt"]): e["n"] for e in run["dispatch_summary"]}
        s2 = {(e["site"], e["a"], e["tgt"]): e["n"] for e in run2["dispatch_summary"]}
        only1 = sorted(set(s1) - set(s2))
        only2 = sorted(set(s2) - set(s1))
        print(f"  跑1 三元组 {len(s1)}，跑2 {len(s2)}；仅跑1 {len(only1)}，仅跑2 {len(only2)}")
        for t in only1[:10]:
            print(f"    仅跑1: site=${t[0]} a={t[1]} tgt=${t[2]}")
        for t in only2[:10]:
            print(f"    仅跑2: site=${t[0]} a={t[1]} tgt=${t[2]}")
        a1 = access_view(run)
        a2 = access_view(run2)
        print(f"  访问集: 跑1 {len(a1)}，跑2 {len(a2)}，并集 {len(a1 | a2)}，交集 {len(a1 & a2)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
