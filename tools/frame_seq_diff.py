#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/frame_seq_diff.py —— 双腿逐帧状态对拍器（机械对拍纪律的执行载体）。

方法论出处：docs/knowledge/pattern-dual-leg-frame-state-compare.md（TD 仪器+
机械对拍四步法）与 gotcha-frame-compare-sampling-staleness.md（三陷阱：采样缝
错位/陈旧产物/空字段集）。

用法：
    python3 tools/frame_seq_diff.py --emu EMU.jsonl --rust RUST.jsonl \
        --field NAME=EMU_SPEC:RUST_SPEC [more --field ...]

    spec 语法（两侧同）：
        KEY          w 字典键；值 int 直接用，str 视为 hex 串取首字节对
        KEY@0xNN     hex 串内 0xNN 字节（如 zp@0x2D=$2D、iram@0x400=$0400）
    例：--field FC=FC:zp@0x2D --field TR=TitleRoutine:zp@0x3E
        （emu w 键 FC/TitleRoutine；rust w.zp 的 0x2D=$2D/0x3E=$1F）

输出：可比帧数、单侧缺行集（缺行=对拍分母不完整，显式列出）、逐字段分歧帧
清单（首/尾）。退出码：0=全同，1=有分歧，2=用法/输入错误。

自测：python3 tools/frame_seq_diff.py --selftest（合成数据 mutation 面自证）。
"""
from __future__ import annotations

import argparse
import json
import sys


def load_rows(path: str, specs: dict[str, tuple[str, str | None]]) -> dict[int, dict[str, int]]:
    """读 JSONL，按 specs[name]=(key, hexoff) 提取每帧字段值（int）。

    hexoff None ⇒ w[key] 直接 int 化（int 或 hex 串取首字节对）；
    hexoff 非 None ⇒ w[key] 视为 hex 串，取 0xNN 字节。
    缺键/越界/非 hex ⇒ 显式 ValueError（禁静默缺字段——假零分歧陷阱）。
    """
    rows: dict[int, dict[str, int]] = {}
    with open(path, encoding="utf-8") as fh:
        for lineno, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            f = rec.get("f")
            w = rec.get("w") or {}
            vals: dict[str, int] = {}
            for name, (key, off) in specs.items():
                if key not in w:
                    raise ValueError(f"{path}:{lineno}: 缺字段 {key!r}"
                                     "（watch 未声明？——空字段集=假零分歧陷阱）")
                v = w[key]
                if isinstance(v, int):
                    if off is not None:
                        raise ValueError(f"{path}:{lineno}: {key} 为 int 不可取偏移")
                    vals[name] = v
                    continue
                if not isinstance(v, str):
                    raise ValueError(f"{path}:{lineno}: {key} 类型不可提取")
                if off is not None:
                    i = int(off, 16) * 2
                    hexb = v[i:i + 2]
                else:
                    hexb = v
                if len(hexb) != 2:
                    raise ValueError(f"{path}:{lineno}: {key} hex 片段异常（{hexb!r}）")
                vals[name] = int(hexb, 16)
            rows[f] = vals
    return rows


def compare(emu: dict, rust: dict) -> dict:
    """固定归因对拍：row[fN] ↔ 帧 fN 状态；逐字段逐帧比较+缺行集。"""
    common = sorted(set(emu) & set(rust))
    missing = {"emu": sorted(set(emu) - set(rust)), "rust": sorted(set(rust) - set(emu))}
    names = sorted({k for v in emu.values() for k in v} |
                   {k for v in rust.values() for k in v})
    div: dict[str, list[int]] = {k: [] for k in names}
    for f in common:
        for k in names:
            a, b = emu[f].get(k), rust[f].get(k)
            if a is not None and b is not None and a != b:
                div[k].append(f)
    return {"common": common, "missing": missing, "divergence": div,
            "counts": {"emu_rows": len(emu), "rust_rows": len(rust),
                       "common": len(common)}}


def parse_field(spec: str) -> tuple[str, tuple[str, str | None], tuple[str, str | None]]:
    """NAME=EMU_KEY[@HEXOFF]:RUST_KEY[@HEXOFF] → (name, (ekey,eoff), (rkey,roff))"""
    name, _, pair = spec.partition("=")
    if not name:
        raise ValueError(f"--field 缺 NAME：{spec!r}")
    left, _, right = pair.partition(":")
    if not right:
        raise ValueError(f"--field 缺 rust 侧：{spec!r}")
    ek, _, eoff = left.partition("@")
    rk, _, roff = right.partition("@")
    if not ek or not rk:
        raise ValueError(f"--field 缺键名：{spec!r}")
    return name, (ek, eoff or None), (rk, roff or None)


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="双腿逐帧状态对拍器")
    ap.add_argument("--emu", required=True)
    ap.add_argument("--rust", required=True)
    ap.add_argument("--field", action="append", required=True,
                    help="NAME=EMU_KEY[:@HEXOFF]:RUST_KEY[@HEXOFF]，可重复")
    a = ap.parse_args(argv[1:])
    try:
        specs = dict(parse_field(sp) for sp in a.field)
        emu = load_rows(a.emu, {n: p[0] for n, p in specs.items()})
        rust = load_rows(a.rust, {n: p[2] for n, p in specs.items()})
    except (ValueError, OSError) as exc:
        print(f"[!] {exc}", file=sys.stderr)
        return 2
    res = compare(emu, rust)
    print(f"可比帧 {res['counts']['common']}（emu {res['counts']['emu_rows']} / "
          f"rust {res['counts']['rust_rows']}）")
    for side in ("emu", "rust"):
        miss = res["missing"][side]
        if miss:
            print(f"[!] {side} 侧缺行 {len(miss)}：{miss[:8]}"
                  f"（缺行=对拍分母不完整，显式列出）")
    ok = True
    for name in sorted(specs):
        d = res["divergence"].get(name) or []
        print(f"{name}: 分歧 {len(d)} 帧"
              + (f"（首 f{d[0]} 尾 f{d[-1]}）" if d else "——✅ 全同"))
        ok = ok and not d
    print("结论：" + ("✅ 全同" if ok else "❌ 有分歧"))
    return 0 if ok else 1


def _selftest() -> int:
    """合成数据 mutation 面自证：已知分歧 ⇒ 必检出；同流 ⇒ 必零。"""
    import json as _json
    import tempfile
    d = tempfile.mkdtemp()
    p1, p2 = d + "/e.jsonl", d + "/r.jsonl"
    with open(p1, "w", encoding="utf-8") as f:
        for i in range(1, 11):
            fc = 1 if i >= 6 else 0            # emu: f6 起 FC=1
            td = 9 if i >= 2 else 0            # emu: f2 起 TD=9
            zp = bytearray(0x100)
            zp[0x1F] = 1 if i >= 3 else 0      # emu: f3 起 TR=1
            f.write(_json.dumps({"f": i, "w": {"FC": fc, "TD": td,
                                               "Z": zp.hex()}}) + "\n")
    with open(p2, "w", encoding="utf-8") as f:
        for i in range(1, 11):
            fc = 1 if i >= 7 else 0            # rust: f7 起 FC=1（晚 1 帧=分歧面）
            td = 9 if i >= 2 else 0            # rust: f2 起 TD=9（同流面——TD 零分歧对照）
            zp = bytearray(0x100)
            zp[0x2D] = fc
            zp[0x1F] = 1 if i >= 3 else 0
            zp[0x29] = td
            f.write(_json.dumps({"f": i, "w": {"zp": zp.hex()}}) + "\n")
    e = load_rows(p1, {"FC": ("FC", None), "TD": ("TD", None), "TR": ("Z", "0x1F")})
    r = load_rows(p2, {"FC": ("zp", "0x2D"), "TD": ("zp", "0x29"),
                       "TR": ("zp", "0x1F")})
    res = compare(e, r)
    fc_div = res["divergence"]["FC"]
    td_div = res["divergence"]["TD"]
    tr_div = res["divergence"]["TR"]
    assert fc_div == [6], f"FC 分歧异常: {fc_div}"
    assert td_div == [], f"TD 应全同: {td_div}"
    assert tr_div == [], f"TR 应全同: {tr_div}"
    print(f"selftest ok（FC 分歧 1 帧@f7 检出；TD/TR 全同零误报）")
    return 0


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        sys.exit(_selftest())
    ap = argparse.ArgumentParser(description="双腿逐帧状态对拍器")
    ap.add_argument("--emu", required=True)
    ap.add_argument("--rust", required=True)
    ap.add_argument("--field", action="append", required=True,
                    help="NAME=EMU_KEY[:@HEXOFF]:RUST_KEY[@HEXOFF]，可重复")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    try:
        specs = {name: (e_pair, r_pair)
                 for name, e_pair, r_pair in (parse_field(sp) for sp in args.field)}
        emu = load_rows(args.emu, {n: p[0] for n, p in specs.items()})
        rust = load_rows(args.rust, {n: p[1] for n, p in specs.items()})
    except (ValueError, OSError) as exc:
        print(f"[!] {exc}", file=sys.stderr)
        sys.exit(2)
    res = compare(emu, rust)
    print(f"可比帧 {res['counts']['common']}（emu {res['counts']['emu_rows']} / "
          f"rust {res['counts']['rust_rows']}）")
    for side in ("emu", "rust"):
        miss = res["missing"][side]
        if miss:
            print(f"[!] {side} 侧缺行 {len(miss)}：{miss[:8]}（缺行=对拍分母不完整）")
    ok = True
    for name in sorted(specs):
        dd = res["divergence"].get(name) or []
        print(f"{name}: 分歧 {len(dd)} 帧"
              + (f"（首 f{dd[0]} 尾 f{dd[-1]}）" if dd else "——✅ 全同"))
        ok = ok and not dd
    print("结论：" + ("✅ 全同" if ok else "❌ 有分歧"))
    sys.exit(0 if ok else 1)
