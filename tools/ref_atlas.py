#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/ref_atlas.py —— task-16 refs 语义线索图谱（三矿开采 + 结构对账）。

背景（用户裁定·已证等价原则）：refs/ 与 ROM 已证字节等价 ⇒ m1disasm/nmikstas 的
结构划分升格为**静态分析主文档**；命名仍是 C 级线索（第三方命名不背书，AGENTS D5），
消费方自证后才升 A/B 入台账。

矿源：
  nmikstas-metroid-disassembly/Source_Files/BankNN.asm —— `Lxxxx` 程序计数标签
    （地址内嵌）+ `.alias Name $ADDR` 语义名 + `.byte/.word` 数据声明 ⇒ 可逐指令
    走址出**代码/数据逐段划分**（bank NN ↔ 我们 bank NN 直接对应）；
  m1disasm/SRC/**.asm —— `Name: ; $XXXX` 富语义标签 + 注释（域=子目录）；
  MetroidMMC3 / nmikstas DOC —— 数据表文档（摘录留 findings）。

子命令：
  reconcile --sweep <decode_sweep --json> [--json out.json]
      结构对账（① 优先）：nmikstas 代码/数据划分 ↔ decode-sweep 岛 verdict 逐段交叉。
      错面确认（ref=code & ours=data）直接喂 task-5③ 分母 v2。
  leads [--json out.json]
      命名候选/边界图谱（②③）：.alias + m1disasm 标签 → 线索表
      （候选名|ref 文件:行|addr|域），与 data/naming/ledger.jsonl 对账命中率。

纪律：每条输出线索标 C 级 + ref 文件:行 指针；本工具只产线索与对账，不 rename。
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NMI_SRC = ROOT / "refs" / "nmikstas-metroid-disassembly" / "Source_Files"
M1_SRC = ROOT / "refs" / "m1disasm" / "SRC"
LEDGER = ROOT / "data" / "naming" / "ledger.jsonl"

OPS2 = None  # decode_sweep 指令表（复用，勿重建）


def _load_ops():
    global OPS2
    if OPS2 is None:
        spec = importlib.util.spec_from_file_location("ds", ROOT / "tools" / "decode_sweep.py")
        ds = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(ds)
        OPS2 = ds.OPS
    return OPS2


def base_of(k: int) -> int:
    return 0xC000 if k == 7 else 0x8000


# ---------------- nmikstas：逐指令走址（代码/数据逐段划分，bank 精确）
RE_LABEL = re.compile(r"^L([0-9A-F]{4}):\s*(.*)$")
RE_ALIAS = re.compile(r"^\.alias\s+(\S+)\s+\$?([0-9A-Fa-f]{2,4})\s*$")
RE_ORG = re.compile(r"^\s*\.org\s+\$?([0-9A-Fa-f]{4})")
RE_BYTE = re.compile(r"^\s*\.byte\s+(.+)$", re.I)
RE_WORD = re.compile(r"^\s*\.word\s+(.+)$", re.I)
MNEM = re.compile(r"^\s*([A-Za-z]{3})\b")


def parse_nmikstas() -> tuple[list[dict], list[dict], list[dict]]:
    """返回 (spans, aliases, divergences)。spans=[{bank,start,end,kind,label,file,line}]"""
    ops = _load_ops()
    spans: list[dict] = []
    aliases: list[dict] = []
    divergences: list[dict] = []
    for fp in sorted(NMI_SRC.glob("Bank*.asm")):
        m = re.match(r"Bank(\d\d)", fp.name)
        if not m:
            continue
        k = int(m.group(1))
        addr = None
        cur: dict | None = None  # 当前累积段

        def close():
            nonlocal cur
            if cur and cur["end"] > cur["start"]:
                spans.append(cur)
            cur = None

        def feed(a: int, n: int, kind: str, label: str | None, line: int):
            nonlocal cur
            if n <= 0:
                return
            if cur is None or cur["kind"] != kind or cur["end"] != a:
                close()
                cur = {"bank": k, "start": a, "end": a + n, "kind": kind,
                       "label": label, "file": f"nmikstas/Source_Files/{fp.name}", "line": line}
            else:
                cur["end"] = a + n

        for ln_no, raw in enumerate(fp.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            line = raw.split(";", 1)[0].rstrip()
            if not line.strip():
                continue
            ma = RE_ALIAS.match(line)
            if ma:
                ALIAS_ADDR[ma.group(1)] = int(ma.group(2), 16)
                aliases.append({"name": ma.group(1), "addr": int(ma.group(2), 16),
                                "bank": k, "file": f"nmikstas/Source_Files/{fp.name}", "line": ln_no})
                continue
            mo = RE_ORG.match(line)
            if mo:
                close()
                addr = int(mo.group(1), 16)
                continue
            ml = RE_LABEL.match(line)
            stmt = ml.group(2) if ml else line
            if ml:
                lab_addr = int(ml.group(1), 16)
                if addr is not None and lab_addr != addr:
                    divergences.append({"bank": k, "label": lab_addr, "walk": addr,
                                        "file": fp.name, "line": ln_no})
                    close()
                    addr = lab_addr
                if addr is None:
                    addr = lab_addr
                label = f"L{ml.group(1)}"
            else:
                label = None
            if addr is None:
                continue
            mb = RE_BYTE.match(stmt)
            mw = RE_WORD.match(stmt)
            if mb:
                n = len([x for x in mb.group(1).split(",") if x.strip()])
                feed(addr, n, "data", label, ln_no)
                addr += n
            elif mw:
                n = 2 * len([x for x in mw.group(1).split(",") if x.strip()])
                feed(addr, n, "data", label, ln_no)
                addr += n
            else:
                # 指令：mnemonic+操作数文本定 mode（#imm/zp/abs/rel…）→ 自建表查长
                op = _match_op(ops, stmt)
                if op:
                    feed(addr, op[2], "code", label, ln_no)
                    addr += op[2]
        close()
    return spans, aliases, divergences


ALIAS_ADDR: dict[str, int] = {}


def _match_op(ops, stmt: str):
    toks = stmt.strip().split(None, 1)
    if not toks:
        return None
    mn = toks[0].lower()
    rest = toks[1].strip() if len(toks) > 1 else ""
    if mn in ("rts", "rti", "tax", "tay", "txa", "tya", "pha", "pla", "php", "plp",
              "inx", "iny", "dex", "dey", "clc", "sec", "cld", "sed", "cli", "sei",
              "clv", "txs", "tsx", "nop", "brk"):
        mode = "imp"
    elif mn in ("asl", "lsr", "rol", "ror") and rest.lower() in ("a", ""):
        mode = "acc"
    elif rest.startswith("#"):
        mode = "imm"
    elif mn in ("jmp",):
        mode = "ind" if rest.startswith("(") else "abs"
    elif mn == "jsr":
        mode = "abs"
    elif mn in ("bpl", "bmi", "bvc", "bvs", "bcc", "bcs", "bne", "beq"):
        mode = "rel"
    elif re.search(r",\s*[xX]\s*$", rest):
        mode = "zpx" if _short_addr(rest) else "abx"
    elif re.search(r",\s*[yY]\s*$", rest):
        mode = "izy" if rest.startswith("(") else ("zpy" if _short_addr(rest) else "aby")
    elif rest.startswith("(") and re.search(r",\s*[xX]\s*\)\s*$", rest):
        mode = "izx"
    elif rest.startswith("("):
        mode = "ind"
    else:
        mode = "zp" if _short_addr(rest) else "abs"
    for _code, (mn2, mode2, ln2) in ops.items():
        if mn2 == mn.upper() and mode2 == mode:
            return (mn2, mode2, ln2)
    return None


def _short_addr(rest: str) -> bool:
    m = re.search(r"\$?([0-9A-Fa-f]{1,2})\b", rest)
    if m and len(m.group(1)) <= 2:
        return True
    # 别名操作数：按 alias 表地址值定 zp/abs（走址分歧 4405 的主因）
    nm = re.match(r"^([A-Za-z_][A-Za-z0-9_]*)", rest.strip())
    if nm and nm.group(1) in ALIAS_ADDR:
        return ALIAS_ADDR[nm.group(1)] < 0x100
    return False


# ---------------- m1disasm：富语义标签（命名候选 + 域）
RE_M1LAB = re.compile(r"^([A-Za-z_@][A-Za-z0-9_@.]*)\s*:\s*;\s*\$([0-9A-F]{4})")


def parse_m1() -> list[dict]:
    out = []
    for fp in sorted(M1_SRC.rglob("*.asm")):
        rel = fp.relative_to(M1_SRC)
        domain = rel.parts[0] if len(rel.parts) > 1 else "root"
        for ln_no, raw in enumerate(fp.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            m = RE_M1LAB.match(raw)
            if m:
                out.append({"name": m.group(1), "addr": int(m.group(2), 16),
                            "file": f"m1disasm/SRC/{rel}", "line": ln_no, "domain": domain})
    return out


# ---------------- ① 结构对账
def cmd_reconcile(args) -> int:
    spans, aliases, divergences = parse_nmikstas()
    sweep = json.loads(Path(args.sweep).read_text(encoding="utf-8"))
    code_spans = [s for s in spans if s["kind"] == "code"]
    data_spans = [s for s in spans if s["kind"] == "data"]

    def cover(k: int, lo: int, hi: int, pool) -> int:
        n = 0
        for s in pool:
            if s["bank"] != k:
                continue
            n += max(0, min(hi, s["end"]) - max(lo, s["start"]))
        return n

    rows = []
    tally = {"错面确认(ref=code)": 0, "升级线索(ref=code)": 0, "边界分歧(ref=data)": 0,
             "对账一致": 0, "ref 无覆盖/混合": 0}
    for isl in sweep["islands"]:
        k, lo, hi, v = isl["bank"], isl["start"], isl["end"], isl["verdict"]
        size = hi - lo
        c, d = cover(k, lo, hi, code_spans), cover(k, lo, hi, data_spans)
        cr, dr = c / size, d / size
        if cr >= 0.9:
            ref = "code"
        elif dr >= 0.9:
            ref = "data"
        else:
            ref = "mixed/none"
        if ref == "code" and v == "data":
            verdict = "错面确认(ref=code)"
        elif ref == "code" and v == "未决":
            verdict = "升级线索(ref=code)"
        elif ref == "data" and v == "code":
            verdict = "边界分歧(ref=data)"
        elif ref == v or (ref == "code" and v == "code"):
            verdict = "对账一致"
        else:
            verdict = "ref 无覆盖/混合"
        tally[verdict] += 1
        if verdict != "ref 无覆盖/混合" and not (verdict == "对账一致" and v == "data"):
            rows.append((f"bank{k:02d}:${lo:04X}-${hi - 1:04X}", v, ref,
                         f"code {cr:.2f}/data {dr:.2f}", verdict))

    print("[ref_atlas] nmikstas 走址：code 段 "
          f"{len(code_spans)} data 段 {len(data_spans)}；走址-标签分歧 {len(divergences)}")
    print("[ref_atlas] 结构对账 tally:", json.dumps(tally, ensure_ascii=False))
    print("\n== 逐岛对账（非 data-data 面）==")
    print(f"{'岛':28s} {'ours':6s} {'ref':10s} {'覆盖':18s} 判定")
    for r in rows:
        print(f"{r[0]:28s} {r[1]:6s} {r[2]:10s} {r[3]:18s} {r[4]}")
    if args.json:
        Path(args.json).write_text(json.dumps({
            "tally": tally,
            "rows": [{"island": r[0], "ours": r[1], "ref": r[2], "cover": r[3], "verdict": r[4]}
                     for r in rows],
            "divergences": divergences,
        }, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"\n[ref_atlas] json → {args.json}")
    return 0


# ---------------- ②③ 线索图谱
def cmd_leads(args) -> int:
    _spans, aliases, _div = parse_nmikstas()
    m1 = parse_m1()
    named_addrs = {}
    if LEDGER.is_file():
        for ln in LEDGER.read_text(encoding="utf-8").splitlines():
            if not ln.strip():
                continue
            try:
                rec = json.loads(ln)
            except json.JSONDecodeError:
                continue
            for a in re.findall(r"\$([0-9A-F]{4})", rec.get("evidence", "") + " " + rec.get("id", "")):
                named_addrs.setdefault(int(a, 16), []).append(rec.get("new") or rec.get("name") or "?")
    print(f"[ref_atlas] 命名候选：nmikstas .alias {len(aliases)} + m1disasm 标签 {len(m1)}")
    print("[ref_atlas] 台账已有地址:", len(named_addrs))
    hit = sum(1 for a in aliases if a["addr"] in named_addrs) + \
          sum(1 for m in m1 if m["addr"] in named_addrs)
    total = len(aliases) + len(m1)
    print(f"[ref_atlas] 候选命中率（与台账地址交集）：{hit}/{total}")
    print("\n== 命名候选线索表（C 级；候选名|ref 指针|addr|域）——节选 ==")
    shown = 0
    for a in aliases:
        if a["addr"] < 0xC000:
            continue
        mark = "已入台账" if a["addr"] in named_addrs else "C 级"
        print(f"  {a['name']:28s} | {a['file']}:{a['line']} | ${a['addr']:04X} | bank{a['bank']:02d} | {mark}")
        shown += 1
        if shown >= 25:
            break
    if args.json:
        Path(args.json).write_text(json.dumps({
            "aliases": aliases, "m1_labels": m1, "ledger_hits": hit, "total": total,
        }, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"\n[ref_atlas] json → {args.json}")
    return 0


def cmd_segs(args) -> int:
    """task-27 巨尾段级切分：nmikstas 走址 span（标签锚定起点）在指定区间内
    同型合并 → 段级 code/data 清单（分母 v2 定值素材）。"""
    spans, _aliases, div = parse_nmikstas()
    lo, hi = int(args.range.split("-")[0], 16), int(args.range.split("-")[1], 16)
    total_div = sum(1 for d in div if lo <= d["label"] < hi)
    segs = {}  # bank -> [(start,end,kind)]
    for sp in spans:
        if sp["kind"] not in ("code", "data"):
            continue
        a, b = max(sp["start"], lo), min(sp["end"], hi)
        if a >= b:
            continue
        L = segs.setdefault(sp["bank"], [])
        if L and L[-1][1] == a and L[-1][2] == sp["kind"]:
            L[-1] = (L[-1][0], b, sp["kind"])
        else:
            L.append((a, b, sp["kind"]))
    print(f"[ref_atlas] 巨尾切分 ${lo:04X}-${hi-1:04X}；区间内走址-标签分歧 {total_div}"
          f"（span 标签锚定起点，段界=锚定点；±漂移如实）")
    grand = {"code": 0, "data": 0}
    for k in sorted(segs):
        code_n = sum(e - a for a, e, t in segs[k] if t == "code")
        data_n = sum(e - a for a, e, t in segs[k] if t == "data")
        grand["code"] += code_n
        grand["data"] += data_n
        n_code = sum(1 for _a, _e, t in segs[k] if t == "code")
        n_data = sum(1 for _a, _e, t in segs[k] if t == "data")
        print(f"  bank{k:02d}: 段 {len(segs[k])}（code {n_code} 段/{code_n}B + "
              f"data {n_data} 段/{data_n}B） 覆盖 {code_n + data_n}B/{hi - lo}B")
    print(f"  合计: code {grand['code']}B data {grand['data']}B（五 bank 摊开）")
    if args.json:
        import json as _j
        out = {str(k): [{"start": a, "end": e, "kind": t} for a, e, t in segs[k]]
               for k in sorted(segs)}
        Path(args.json).write_text(_j.dumps(out, indent=1), encoding="utf-8")
        print(f"[ref_atlas] json → {args.json}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="task-16 refs 语义线索图谱")
    sub = ap.add_subparsers(dest="cmd", required=True)
    r1 = sub.add_parser("reconcile")
    r1.add_argument("--sweep", required=True)
    r1.add_argument("--json", default=None)
    r2 = sub.add_parser("leads")
    r2.add_argument("--json", default=None)
    r3 = sub.add_parser("segs")
    r3.add_argument("--range", default="8D3A-BFFF")
    r3.add_argument("--json", default=None)
    args = ap.parse_args()
    if args.cmd == "reconcile":
        return cmd_reconcile(args)
    if args.cmd == "leads":
        return cmd_leads(args)
    return cmd_segs(args)


if __name__ == "__main__":
    import sys
    sys.exit(main())
