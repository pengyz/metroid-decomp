#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/decode_sweep.py —— seed 数据岛 × ROM 原字节解码缺口扫描器（decode-sweep）。

任务（docs/findings/seed-decode-gap-f0a2-f148.md 处置①）：系统性找出 seed 反汇编中
所有「数据岛实为代码」的解码缺口。

原则（AGENTS.md §3 / gotcha-listing-grep-not-rom-scan / gotcha-operand-type-by-opcode-byte）：
  - 判据全部落在 **ROM 原字节**上：PRG 直接读取，6502 指令长度表**自建**（下表），
    操作数类型以**操作码字节**为准；listing 只提供「数据岛区间标注 / 代码区跨度 /
    标签锚点」，且每行 hex 逐字节回核 PRG（不一致=硬失败）。
  - 否定性结论（verdict=data）双工具复核（证据铁律）：入口驱动扫描之外再做
    **全偏移盲扫**；两法都无覆盖级命中才判 data。

判据（任务书 ①②③；④循环同构为可选强判据，v1 未实现，见报告遗留）：
  ① 指令流连贯至 RTS/RTI/JMP——终止指令可在岛内也可在相邻区（岛边界是 seed
     标注不是语义边界；实证案例 F282/F2ED 流跨标注才到 RTS/JMP）；BRK/非法操作码/
     512 条未终止 = 流不成立；
  ② 分支目标分类核查：指令边界/锚点=强通过；数据岛内=缺口信号（送入口候选）；
     代码区内部=弱通过（6502 跳进操作数的合法技巧）；RAM/未映射=失败；
  ③ JSR/JMP(abs) 目标同 ② 分类（JMP 间接的操作数是指针存放处，不作代码目标核查）。
  调入数据岛=解码缺口信号：登记调用点并给目标岛送入口候选（F0A2→F162 类）。

入口佐证（verdict 分级依据）：fallthrough-from-code / xref-from-code /
stream-xref（被采纳流调用/分支进来）= 有佐证；仅 island-start = 无佐证。

回归基准（环⑤：测试器先过证伪——正/负对照都来自已定谳 A 级证据）：
  正对照 bank07:$F0A2-$F148 → verdict=code 且复现 9 个在岛调用点
    （findings/seed-decode-gap-f0a2-f148.md；ROM 字节 20 62 F1 @ $F109 等）；
  负对照（真数据）：$F018 00 55 AA FF 图案填充色 / $FC5D 指针表 /
    $FF86 索引表 / $FFE9 "METROID" ASCII——所在岛必须判 data。

用法：
  python3 tools/decode_sweep.py [--json out.json] [--matrix data/status/completeness_matrix.json]
                                [--no-baseline] [--verbose]
  只读工具：不写仓库文件（--json 由调用者指定落点）。

输出：每岛 verdict（code/data/未决）+ 依据（入口/流覆盖/终止点/弱链）；
  新增调用点清单（bank:addr → target）；S/D 分母影响素材（错面格/少计入口，
  分母登记归 lead，本工具不动 completeness_denoms.py 的产物）。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROM_PATH = ROOT / "rom" / "Metroid (USA).nes"
SEED_PATH = ROOT / "reference" / "seed" / "metroid.asm"
ROM_SHA256 = "e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6"

N_BANKS = 8
BANK_SIZE = 0x4000
PRG_SIZE = 0x20000
MAX_INSTRS = 512          # ① 未终止流的上限（防垃圾流漫游半 bank）
CODE_COVER = 0.9          # verdict=code 的岛内覆盖率阈值
BRUTE_COVER = 0.5         # 盲扫命中升级未决的岛内覆盖率阈值


def bank_base(k: int) -> int:
    return 0xC000 if k == 7 else 0x8000


# ---------------------------------------------------------------------------
# 自建 6502 指令长度表（官方 151 条；操作数类型以操作码字节为准）
# 非官方操作码与 BRK 一律视为流不成立——任天堂官方代码不用；数据岛垃圾字节
# 大量落在其上，此严格性是判别力来源之一（负对照 4/4 拒绝即其证据）。
# ---------------------------------------------------------------------------
_OFFICIAL_ROWS = """
00 BRK imp; 01 ORA izx; 05 ORA zp; 06 ASL zp; 08 PHP imp; 09 ORA imm; 0A ASL acc;
0D ORA abs; 0E ASL abs; 10 BPL rel; 11 ORA izy; 15 ORA zpx; 16 ASL zpx; 18 CLC imp;
19 ORA aby; 1D ORA abx; 1E ASL abx; 20 JSR abs; 21 AND izx; 24 BIT zp; 25 AND zp;
26 ROL zp; 28 PLP imp; 29 AND imm; 2A ROL acc; 2C BIT abs; 2D AND abs; 2E ROL abs;
30 BMI rel; 31 AND izy; 35 AND zpx; 36 ROL zpx; 38 SEC imp; 39 AND aby; 3D AND abx;
3E ROL abx; 40 RTI imp; 41 EOR izx; 45 EOR zp; 46 LSR zp; 48 PHA imp; 49 EOR imm;
4A LSR acc; 4C JMP abs; 4D EOR abs; 4E LSR abs; 50 BVC rel; 51 EOR izy; 55 EOR zpx;
56 LSR zpx; 58 CLI imp; 59 EOR aby; 5D EOR abx; 5E LSR abx; 60 RTS imp; 61 ADC izx;
65 ADC zp; 66 ROR zp; 68 PLA imp; 69 ADC imm; 6A ROR acc; 6C JMP ind; 6D ADC abs;
6E ROR abs; 70 BVS rel; 71 ADC izy; 75 ADC zpx; 76 ROR zpx; 78 SEI imp; 79 ADC aby;
7D ADC abx; 7E ROR abx; 81 STA izx; 84 STY zp; 85 STA zp; 86 STX zp; 88 DEY imp;
8A TXA imp; 8C STY abs; 8D STA abs; 8E STX abs; 90 BCC rel; 91 STA izy; 94 STY zpx;
95 STA zpx; 96 STX zpy; 98 TYA imp; 99 STA aby; 9A TXS imp; 9D STA abx; A0 LDY imm;
A1 LDA izx; A2 LDX imm; A4 LDY zp; A5 LDA zp; A6 LDX zp; A8 TAY imp; A9 LDA imm;
AA TAX imp; AC LDY abs; AD LDA abs; AE LDX abs; B0 BCS rel; B1 LDA izy; B4 LDY zpx;
B5 LDA zpx; B6 LDX zpy; B8 CLV imp; B9 LDA aby; BA TSX imp; BC LDY abx; BD LDA abx;
BE LDX aby; C0 CPY imm; C1 CMP izx; C4 CPY zp; C5 CMP zp; C6 DEC zp; C8 INY imp;
C9 CMP imm; CA DEX imp; CC CPY abs; CD CMP abs; CE DEC abs; D0 BNE rel; D1 CMP izy;
D5 CMP zpx; D6 DEC zpx; D8 CLD imp; D9 CMP aby; DD CMP abx; DE DEC abx; E0 CPX imm;
E1 SBC izx; E4 CPX zp; E5 SBC zp; E6 INC zp; E8 INX imp; E9 SBC imm; EA NOP imp;
EC CPX abs; ED SBC abs; EE INC abs; F0 BEQ rel; F1 SBC izy; F5 SBC zpx; F6 INC zpx;
F8 SED imp; F9 SBC aby; FD SBC abx; FE INC abx
"""

MODE_LEN = {"imp": 1, "acc": 1, "imm": 2, "zp": 2, "zpx": 2, "zpy": 2,
            "izx": 2, "izy": 2, "rel": 2, "abs": 3, "abx": 3, "aby": 3, "ind": 3}
BRANCHES = {"BPL", "BMI", "BVC", "BVS", "BCC", "BCS", "BNE", "BEQ"}
TERMINATORS = {"RTS", "RTI", "JMP"}


def _build_ops() -> dict[int, tuple[str, str, int]]:
    ops: dict[int, tuple[str, str, int]] = {}
    for row in _OFFICIAL_ROWS.replace("\n", " ").split(";"):
        row = row.strip()
        if not row:
            continue
        op_hex, mn, mode = row.split()
        ops[int(op_hex, 16)] = (mn, mode, MODE_LEN[mode])
    return ops


OPS = _build_ops()
assert len(OPS) == 151, f"official opcode table broken: {len(OPS)}"

# 回归基准（findings/seed-decode-gap-f0a2-f148.md 等，A 级 ROM 字节）
BASELINE_CODE_ISLAND_ADDR = (7, 0xF0A2)   # 正对照：命中所在岛即比对
BASELINE_CALL_SITES = {
    (7, 0xF0A8): 0xF152, (7, 0xF0C9): 0xF140, (7, 0xF0DF): 0xF140,
    (7, 0xF106): 0xF1B3, (7, 0xF109): 0xF162, (7, 0xF10C): 0xF1FA,
    (7, 0xF120): 0xF186, (7, 0xF140): 0xF1BF, (7, 0xF143): 0xF186,
}
BASELINE_DATA_ADDRS = [  # 负对照：真数据（所在岛必须判 data）
    (7, 0xF018),  # 00 55 AA FF 图案填充色（F012 循环写 PPU $C000）
    (7, 0xFC5D),  # 指针表（FBF5 LDA $FC5D,Y）
    (7, 0xFF86),  # 索引表（FF66 LDA $FF86,Y）
    (7, 0xFFE9),  # "METROID" ASCII
]


# ---------------------------------------------------------------------------
# ROM / seed 载入
# ---------------------------------------------------------------------------
class Bank:
    def __init__(self, k: int, data: bytes):
        self.idx = k
        self.base = bank_base(k)
        self.data = data

    def off(self, cpu_addr: int) -> int:
        return cpu_addr - self.base

    def byte(self, cpu_addr: int) -> int:
        return self.data[self.off(cpu_addr)]

    def word(self, cpu_addr: int) -> int:
        o = self.off(cpu_addr)
        return self.data[o] | (self.data[o + 1] << 8)


def load_prg() -> dict[int, Bank]:
    rom = ROM_PATH.read_bytes()
    sha = hashlib.sha256(rom).hexdigest()
    if sha != ROM_SHA256:
        raise SystemExit(f"FATAL: rom sha256 不符（期望 {ROM_SHA256}，实得 {sha}）——ROM 身份锚点失守")
    prg = rom[0x10:0x10 + PRG_SIZE]
    if len(prg) != PRG_SIZE:
        raise SystemExit(f"FATAL: PRG 长度 {len(prg):#x} != {PRG_SIZE:#x}")
    return {k: Bank(k, prg[k * BANK_SIZE:(k + 1) * BANK_SIZE]) for k in range(N_BANKS)}


SEED_LINE = re.compile(r"^(bank\d\d):([0-9A-F]{4})  (.+)$")
DATA_TEXT = re.compile(r"^\$[0-9A-F]{2}(,\$[0-9A-F]{2})*$")
LABEL = re.compile(r"\b(?:sub|loc|byte|unk|off|pt)_([0-9A-F]{4})\b")


def parse_seed(banks: dict[int, Bank]):
    """listing 只取三样标注：数据岛区间、代码区跨度、标签锚点；字节一律回读 PRG。"""
    islands: dict[int, list[tuple[int, int]]] = {k: [] for k in range(N_BANKS)}
    code_items: dict[int, list[tuple[int, int]]] = {k: [] for k in range(N_BANKS)}
    labels: dict[int, set[int]] = {k: set() for k in range(N_BANKS)}
    cur: dict[int, list[int] | None] = {k: None for k in range(N_BANKS)}

    def flush(k: int):
        if cur[k] is not None:
            islands[k].append((cur[k][0], cur[k][1]))
            cur[k] = None

    for ln_no, line in enumerate(SEED_PATH.read_text(encoding="utf-8").splitlines(), 1):
        m = SEED_LINE.match(line)
        if not m:
            continue
        k = int(m.group(1)[4:])
        addr = int(m.group(2), 16)
        parts = re.split(r"\s{2,}", m.group(3), maxsplit=1)
        hx = parts[0].split()
        text = parts[1].strip() if len(parts) > 1 else ""
        raw = bytes(int(b, 16) for b in hx)
        prg_slice = banks[k].data[banks[k].off(addr): banks[k].off(addr) + len(raw)]
        if prg_slice != raw:
            raise SystemExit(f"FATAL: seed 行 {ln_no} bank{k:02d}:{addr:04X} hex 与 PRG 原字节不符")
        if DATA_TEXT.match(text):
            if cur[k] is not None and cur[k][1] == addr:
                cur[k][1] = addr + len(raw)
            else:
                flush(k)
                cur[k] = [addr, addr + len(raw)]
        else:
            flush(k)
            code_items[k].append((addr, len(raw)))
            for lab in LABEL.findall(text):
                labels[k].add(int(lab, 16))
    for k in range(N_BANKS):
        flush(k)
    return islands, code_items, labels


# ---------------------------------------------------------------------------
# 线性反汇编尝试（ROM 原字节 + 自建长度表）
# ---------------------------------------------------------------------------
class Sweep:
    def __init__(self, bank: int, entry: int):
        self.bank = bank
        self.entry = entry
        self.instrs: list[dict] = []
        self.boundaries: set[int] = set()
        self.terminator: tuple[int, str] | None = None
        self.fail: str | None = None
        self.weak: list[str] = []            # 弱链（岛内/代码区内部目标）
        self.island_targets: list[int] = []  # 落数据岛的分支/跳转/调用目标

    @property
    def ok(self) -> bool:
        return self.fail is None and self.terminator is not None


def sweep_linear(banks: dict[int, Bank], k: int, entry: int,
                 max_instrs: int = MAX_INSTRS) -> Sweep:
    """① 从 entry 直线解码至 RTS/RTI/JMP（可跨出岛标注）；BRK/非法/超长=失败。"""
    bk = banks[k]
    sw = Sweep(k, entry)
    pc = entry
    for _ in range(max_instrs):
        if pc < bk.base or pc >= bk.base + BANK_SIZE:
            sw.fail = f"pc ${pc:04X} 越出 bank 窗口"
            return sw
        op = bk.byte(pc)
        if op == 0x00:  # BRK 不作代码流（判别力来源；见模块头）
            sw.fail = f"BRK @ ${pc:04X}（BRK 不作代码流）"
            return sw
        hit = OPS.get(op)
        if hit is None:
            sw.fail = f"非法/未授权操作码 ${op:02X} @ ${pc:04X}" + \
                      ("（BRK 不作代码流）" if op == 0x00 else "")
            return sw
        mn, mode, ln = hit
        if pc + ln > bk.base + BANK_SIZE:
            sw.fail = f"指令跨出 bank 尾 @ ${pc:04X}"
            return sw
        target = None
        if mode == "rel":
            rel = bk.byte(pc + 1)
            target = pc + ln + (rel - 0x100 if rel >= 0x80 else rel)
        elif mode in ("abs", "abx", "aby"):
            target = bk.word(pc + 1)
        # mode == "ind"：JMP 间接，操作数是指针存放处，不作代码目标核查
        sw.instrs.append({"addr": pc, "op": op, "mn": mn, "mode": mode,
                          "len": ln, "target": target})
        sw.boundaries.add(pc)
        if mn in TERMINATORS:
            sw.terminator = (pc, mn)
            return sw
        pc += ln
    sw.fail = f"{max_instrs} 条未见 RTS/RTI/JMP（过长未终止）"
    return sw


# ---------------------------------------------------------------------------
# 目标分类（②③）与岛扫描
# ---------------------------------------------------------------------------
def find_island(islands: list[tuple[int, int]], addr: int):
    for lo, hi in islands:
        if lo <= addr < hi:
            return (lo, hi)
    return None


def find_code_span(code_items: list[tuple[int, int]], addr: int):
    for a, n in code_items:
        if a <= addr < a + n:
            return (a, a + n)
    return None


class Classifier:
    """目标分类：boundary / anchor / island / codespan / bad（RAM/未映射）。"""

    def __init__(self, banks, islands, code_items, labels):
        self.banks = banks
        self.islands = islands
        self.code_items = code_items
        self.anchors = {k: {a for a, _n in code_items[k]} | labels[k] for k in range(N_BANKS)}
        self.win_anchor: set[int] = set()
        self.win_islands: list[tuple[int, int]] = []
        self.win_spans: list[tuple[int, int]] = []
        for k in range(6):
            self.win_anchor |= self.anchors[k]
            self.win_islands.extend(islands[k])
            self.win_spans.extend((a, a + n) for a, n in code_items[k])

    def classify(self, k: int, t: int, boundaries: set[int]):
        if t in boundaries:
            return ("boundary", None)
        if t in self.anchors[k]:
            return ("anchor", None)
        isl = find_island(self.islands[k], t)
        if isl:
            return ("island", isl)
        span = find_code_span(self.code_items[k], t)
        if span:
            return ("codespan", span)
        if 0x8000 <= t <= 0xBFFF:  # 跨 bank 窗口目标：banks0-6 并集语义
            if t in self.win_anchor:
                return ("anchor", None)
            for lo, hi in self.win_islands:
                if lo <= t < hi:
                    return ("island", (lo, hi))
            for lo, hi in self.win_spans:
                if lo <= t < hi:
                    return ("codespan", (lo, hi))
        return ("bad", None)


def scan(banks, islands, code_items, labels):
    cls = Classifier(banks, islands, code_items, labels)

    # ---- 入口候选：island-start / 代码区 xref（ROM 字节扫描）/ 顺流 ----
    cands: dict[tuple[int, int], set[str]] = {}
    for k in range(N_BANKS):
        bk = banks[k]
        for lo, hi in islands[k]:
            cands.setdefault((k, lo), set()).add("island-start")
        for a, n in code_items[k]:
            if find_island(islands[k], a) is not None:
                continue
            op = bk.byte(a)
            hit = OPS.get(op)
            if not hit:
                continue
            mn, mode, ln = hit
            t = None
            if mode == "rel":
                rel = bk.byte(a + 1)
                t = a + ln + (rel - 0x100 if rel >= 0x80 else rel)
            elif mn in ("JSR", "JMP") and mode == "abs":
                t = bk.word(a + 1)
            if t is not None and find_island(islands[k], t) is not None:
                cands.setdefault((k, t), set()).add(f"xref-from-code@{a:04X}")
            if find_island(islands[k], a + n) is not None:
                cands.setdefault((k, a + n), set()).add(f"fallthrough-from-code@{a:04X}")

    # ---- 入口不动点：采纳流贡献流 xref 入口 ----
    accepted: dict[tuple[int, int], Sweep] = {}
    rejected: dict[tuple[int, int], str] = {}
    pending = set(cands)
    while pending:
        key = min(pending)
        pending.discard(key)
        if key in accepted or key in rejected:
            continue
        k, entry = key
        if find_island(islands[k], entry) is None:
            continue
        sw = sweep_linear(banks, k, entry)
        if not sw.ok:
            rejected[key] = sw.fail or "① 不成立"
            continue
        bad = []
        for ins in sw.instrs:  # ②③ 目标分类核查（弱链登记，不拖垮调用者）
            t, mn, mode = ins["target"], ins["mn"], ins["mode"]
            if t is None:
                continue
            if mn in BRANCHES or (mn in ("JSR", "JMP") and mode == "abs"):
                kind, _info = cls.classify(k, t, sw.boundaries)
                if kind == "bad":
                    bad.append(f"{mn}@${ins['addr']:04X}→${t:04X}(RAM/未映射)")
                elif kind == "island":
                    sw.weak.append(f"{mn}@${ins['addr']:04X}→${t:04X}@island")
                    sw.island_targets.append(t)
                    cands.setdefault((k, t), set()).add(f"stream-xref-from-{entry:04X}")
                    pending.add((k, t))
                elif kind == "codespan":
                    sw.weak.append(f"{mn}@${ins['addr']:04X}→${t:04X}@codespan-mid")
        if bad:
            rejected[key] = "②③ 目标失败：" + ",".join(bad[:3])
            continue
        accepted[key] = sw

    # ---- 岛 verdict（覆盖=被采纳且有佐证流的指令字节）----
    justified = {key for key, src in cands.items()
                 if any(s != "island-start" for s in src)}
    results = []
    call_sites: list[tuple[int, int, int, str]] = []
    called_islands: set[tuple[int, int, int]] = set()
    for k in range(N_BANKS):
        for lo, hi in islands[k]:
            cov: set[int] = set()
            cov_blind: set[int] = set()
            entries = []
            terms: list[str] = []
            weak_open: list[str] = []
            for (k2, e), sw in sorted(accepted.items()):
                if k2 != k:
                    continue
                # 覆盖按指令**字节**计（3 字节 JSR 不能只算首址）
                hits = {a for ins in sw.instrs
                        for a in range(ins["addr"], ins["addr"] + ins["len"])
                        if lo <= a < hi}
                if not hits:
                    continue
                is_just = (k2, e) in justified
                entries.append({"entry": e, "justified": is_just, "cover": len(hits)})
                if is_just:
                    cov |= set(hits)
                    if sw.terminator:
                        terms.append(f"${sw.terminator[0]:04X}:{sw.terminator[1]}")
                    for w in sw.weak:
                        if "island" in w:
                            weak_open.append(f"${e:04X}:{w}")
                else:
                    cov_blind |= set(hits)
                for ins in sw.instrs:
                    if not (lo <= ins["addr"] < hi):
                        continue
                    if ins["mn"] == "JSR" and ins["mode"] == "abs":
                        call_sites.append((k, ins["addr"], ins["target"], "JSR"))
                    elif ins["mn"] == "JMP" and ins["mode"] == "abs":
                        call_sites.append((k, ins["addr"], ins["target"], "JMP"))
            size = hi - lo
            ratio = len(cov) / size if size else 0.0
            blind_ratio = len(cov_blind) / size if size else 0.0
            if ratio >= CODE_COVER:
                verdict = "code"
            elif ratio > 0 or blind_ratio >= BRUTE_COVER:
                verdict = "未决"
            else:
                verdict = "data"
            results.append({
                "bank": k, "start": lo, "end": hi, "size": size,
                "verdict": verdict, "coverage": round(ratio, 3),
                "blind_coverage": round(blind_ratio, 3),
                "entries": entries,
                "fail_reasons": {f"${e:04X}": r for (k2, e), r in sorted(rejected.items())
                                 if k2 == k and lo <= e < hi},
                "terminators": sorted(set(terms)),
                "weak_links": sorted(set(weak_open))[:8],
            })

    # 代码区 xref 入岛（缺口强信号，供报告）
    for k in range(N_BANKS):
        bk = banks[k]
        for a, n in code_items[k]:
            op = bk.byte(a)
            hit = OPS.get(op)
            if not hit:
                continue
            mn, mode, _ln = hit
            if not (mn in ("JSR", "JMP") and mode == "abs"):
                continue
            t = bk.word(a + 1)
            isl = find_island(islands[k], t)
            if isl:
                called_islands.add((k, isl[0], isl[1]))

    # ---- 否定结论换工具复核：data 岛全偏移盲扫（①②③ 同判据）----
    brute_stats: dict[tuple[int, int], tuple[float, int | None]] = {}
    for r in results:
        if r["verdict"] != "data":
            continue
        k, lo, hi = r["bank"], r["start"], r["end"]
        best, best_off = 0.0, None
        for off in range(lo, hi):
            sw = sweep_linear(banks, k, off)
            if not sw.ok:
                continue
            bad = False
            for ins in sw.instrs:
                t, mn, mode = ins["target"], ins["mn"], ins["mode"]
                if t is None:
                    continue
                if mn in BRANCHES or (mn in ("JSR", "JMP") and mode == "abs"):
                    kind, _info = cls.classify(k, t, sw.boundaries)
                    if kind == "bad":
                        bad = True
                        break
            if bad:
                continue
            cov = len({a for ins in sw.instrs
                       for a in range(ins["addr"], ins["addr"] + ins["len"])
                       if lo <= a < hi}) / (hi - lo)
            if cov > best:
                best, best_off = cov, off
        brute_stats[(k, lo)] = (best, best_off)
        if best >= BRUTE_COVER:
            r["verdict"] = "未决"
            r["blind_coverage"] = round(best, 3)
            r["fail_reasons"]["bruteforce"] = (
                f"盲扫命中入口 ${best_off:04X} 覆盖 {best:.2f}（无 xref 佐证）→ 未决")
        else:
            r["fail_reasons"]["bruteforce"] = f"全偏移盲扫最大覆盖 {best:.2f}（< {BRUTE_COVER}）"

    return (results, sorted(set(call_sites)), cands, accepted, rejected,
            brute_stats, called_islands)


# ---------------------------------------------------------------------------
# 回归基准（环⑤：正/负对照先证判别力，再信它的绿）
# ---------------------------------------------------------------------------
def check_baseline(results, call_sites, islands) -> list[str]:
    errs = []
    bk, addr = BASELINE_CODE_ISLAND_ADDR
    isl = find_island(islands[bk], addr)
    hit = [r for r in results if (r["bank"], r["start"], r["end"]) == (bk, isl[0], isl[1])] if isl else []
    if not hit:
        errs.append(f"正对照缺失：bank{bk:02d}:${addr:04X} 所在岛不在结果中")
    elif hit[0]["verdict"] != "code":
        errs.append(f"正对照失败：bank{bk:02d}:${isl[0]:04X}-${isl[1] - 1:04X} "
                    f"verdict={hit[0]['verdict']}（期望 code）coverage={hit[0]['coverage']}")
    found = {(k, a): t for k, a, t, kind in call_sites if kind == "JSR"}
    for (k, a), t in sorted(BASELINE_CALL_SITES.items()):
        got = found.get((k, a))
        if got != t:
            errs.append(f"调用点未复现：bank{k:02d}:${a:04X} JSR ${t:04X}（实得 {got}）")
    for k, addr in BASELINE_DATA_ADDRS:
        isl = find_island(islands[k], addr)
        hit = [r for r in results if (r["bank"], r["start"], r["end"]) == (k, isl[0], isl[1])] if isl else []
        if not hit:
            errs.append(f"负对照缺失：bank{k:02d}:${addr:04X} 所在岛")
        elif hit[0]["verdict"] != "data":
            errs.append(f"负对照失败：bank{k:02d}:${addr:04X} 所在岛 "
                        f"verdict={hit[0]['verdict']}（期望 data）——判别力不足")
    return errs


# ---------------------------------------------------------------------------
# S/D 分母影响素材（只读矩阵；登记归 lead）
# ---------------------------------------------------------------------------
def denom_impact(results, justified_ok: set, matrix_path: Path | None):
    if not matrix_path or not matrix_path.is_file():
        return None
    m = json.loads(matrix_path.read_text(encoding="utf-8"))
    cells = m.get("cells", {})
    dregs, sfuncs = [], set()
    for cid, c in cells.items():
        mm = re.match(r"^bank(\d\d):([0-9A-F]+)-([0-9A-F]+)", c.get("evidence", ""))
        if cid.startswith("D-REG-") and mm:
            dregs.append((cid, int(mm.group(1)), int(mm.group(2), 16),
                          int(mm.group(3), 16), c.get("note", "")))
        if cid.startswith("S-FUNC-"):
            sfuncs.add(cid)
    wrong_face = []
    for r in results:
        if r["verdict"] == "data":
            continue
        for cid, dk, ds, de, note in dregs:
            if dk == r["bank"] and not (de <= r["start"] or ds >= r["end"]):
                wrong_face.append({
                    "cell": cid,
                    "range": f"bank{dk:02d}:${ds:04X}-${de - 1:04X}",
                    "island": f"bank{r['bank']:02d}:${r['start']:04X}-${r['end'] - 1:04X}",
                    "verdict": r["verdict"],
                    "already_annotated": "错面登记" in note,
                })
    s_missing = sorted(f"bank{k:02d}:${e:04X}" for (k, e) in justified_ok
                       if f"S-FUNC-{k:02d}-{e:04X}" not in sfuncs)
    return {"wrong_face_d_cells": wrong_face,
            "s_face_missing_entries": s_missing,
            "s_face_missing_count": len(s_missing)}


# ---------------------------------------------------------------------------
def main() -> int:
    ap = argparse.ArgumentParser(description="seed 数据岛 × ROM 原字节解码缺口扫描")
    ap.add_argument("--json", type=Path, default=None)
    ap.add_argument("--matrix", type=Path,
                    default=ROOT / "data" / "status" / "completeness_matrix.json")
    ap.add_argument("--no-baseline", action="store_true")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    banks = load_prg()
    islands, code_items, labels = parse_seed(banks)
    n_isl = sum(len(v) for v in islands.values())
    n_bytes = sum(hi - lo for v in islands.values() for lo, hi in v)
    print(f"[decode-sweep] rom sha256 ok; seed 数据岛 {n_isl} 个 / {n_bytes} 字节")

    results, call_sites, cands, accepted, rejected, brute, called_islands = scan(
        banks, islands, code_items, labels)

    by_v = {"code": 0, "data": 0, "未决": 0}
    for r in results:
        by_v[r["verdict"]] += 1
    n_brute_up = sum(1 for r in results if "bruteforce" in r["fail_reasons"]
                     and "盲扫命中" in r["fail_reasons"]["bruteforce"])
    print(f"[decode-sweep] verdict：code={by_v['code']}  data={by_v['data']}  未决={by_v['未决']}"
          f"（岛 {n_isl}；被采纳流 {len(accepted)}，被拒 {len(rejected)}，"
          f"入口候选 {len(cands)}；盲扫升级 {n_brute_up}）")

    print("\n== verdict=code 岛（解码缺口）==")
    for r in results:
        if r["verdict"] != "code":
            continue
        just = [f"${e['entry']:04X}" for e in r["entries"] if e["justified"]]
        print(f"bank{r['bank']:02d}:${r['start']:04X}-${r['end'] - 1:04X} ({r['size']}B) "
              f"coverage={r['coverage']} 佐证入口={just} term={r['terminators']}")
    print("\n== verdict=未决 岛（待人工复核）==")
    for r in results:
        if r["verdict"] != "未决":
            continue
        ents = [f"${e['entry']:04X}" for e in r["entries"]]
        print(f"bank{r['bank']:02d}:${r['start']:04X}-${r['end'] - 1:04X} ({r['size']}B) "
              f"coverage={r['coverage']} blind={r['blind_coverage']} entries={ents}")
        if args.verbose:
            for e, why in r["fail_reasons"].items():
                print(f"    {e}: {why}")

    print("\n== 新增调用点清单（岛内被采纳流，bank:addr → target）==")
    jsr = [(k, a, t) for k, a, t, kind in call_sites if kind == "JSR"]
    jmp = [(k, a, t) for k, a, t, kind in call_sites if kind == "JMP"]
    for k, a, t in sorted(jsr):
        print(f"  bank{k:02d}:${a:04X}  JSR ${t:04X}")
    print(f"  （JSR {len(jsr)} 处；JMP {len(jmp)} 处："
          + ", ".join(f"bank{k:02d}:${a:04X}→${t:04X}" for k, a, t in sorted(jmp)) + "）")

    print("\n== 否定结论换工具复核（data 岛全偏移盲扫）==")
    worst = sorted(((v[0], k) for k, v in brute.items()), reverse=True)[:5]
    print(f"  data 岛 {by_v['data']} 个全部经盲扫复核；升级未决 {n_brute_up} 个；"
          f"最大残余覆盖 top5："
          + ", ".join(f"bank{k[0]:02d}:${k[1]:04X}={v:.2f}" for v, k in worst))

    justified_ok = {(k, e) for (k, e), sw in accepted.items()
                    if any(s != "island-start" for s in cands.get((k, e), set()))}
    impact = denom_impact(results, justified_ok, args.matrix)
    if impact:
        print("\n== S/D 分母影响素材（只读矩阵；登记归 lead）==")
        for w in impact["wrong_face_d_cells"]:
            mark = "已注记" if w["already_annotated"] else "待注记"
            tag = "错面确定" if w["verdict"] == "code" else "疑似(未决)"
            print(f"  {tag} {w['cell']} {w['range']} ⊇ {w['island']} "
                  f"verdict={w['verdict']} [{mark}]")
        print(f"  S 面少计估计：有佐证岛内入口不在 S-FUNC 分母 = "
              f"{impact['s_face_missing_count']} 个："
              + ", ".join(impact["s_face_missing_entries"][:20])
              + (" …" if impact["s_face_missing_count"] > 20 else ""))

    if args.json:
        args.json.write_text(json.dumps({
            "islands": results,
            "call_sites": [{"bank": k, "addr": a, "target": t, "kind": kind}
                           for k, a, t, kind in sorted(call_sites)],
            "called_islands": [f"bank{k:02d}:${lo:04X}-${hi - 1:04X}"
                               for k, lo, hi in sorted(called_islands)],
            "denom_impact": impact,
            "counts": {**by_v, "islands": n_isl, "island_bytes": n_bytes,
                       "bruteforce_upgrades": n_brute_up},
        }, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"\n[decode-sweep] json → {args.json}")

    if not args.no_baseline:
        print("\n== 回归基准（正对照 F0A2-F148 + 9 调用点 / 4 负对照）==")
        errs = check_baseline(results, call_sites, islands)
        if errs:
            for e in errs:
                print(f"  FAIL {e}")
            print("BASELINE: FAIL")
            return 1
        print("  正对照 bank07:$F0A2-$F148 所在岛 → code ✓  9/9 调用点复现 ✓")
        print("  负对照 4/4 所在岛 → data ✓（判别力：已知缺口 vs 已知数据全对）")
        print("BASELINE: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
