#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tools/routine_inventory.py —— 例程清单（结构驱动迁移的分母）。

目的：把「还缺什么」从「L4 哪一格不对就挖哪格」变成**机械可查的清单**。

分母来源是 `refs/m1disasm/SRC/**/*.asm` 的 **源码级 label**（不是每个 bank 的实例——
同一例程会被 5 个区域 bank 各 include 一次，按实例计数会把分母放大 5 倍且毫无信息量）。
每条 label 按「下一条非空行是不是数据指示（`.byte/.word/.dw/...`）」判定 code/data：

- **code**  label = 例程（本清单的分子分母都只算它）；
- **data**  label = 数据表（已由资产包 `pack-tool verify` 逐段自证，这里只登记不计入缺口）。

code label 的状态三档：
- `ported`    —— `data/status/routine_inventory_overrides.json` 里显式登记 Rust 符号
                 （人工/考证过的映射，比名字匹配可信）；
- `candidate` —— label 的 snake_case 与某个 Rust `fn` 同名（**线索不是结论**：同名不等于
                 行为等价，按 asm 复核后才能升 `ported`）；
- `unported`  —— 都没有。

`--check` 供门禁用：与已落盘清单比对，**unported 只减不增**，且源码集变化必须显式重生成。

用法：
    python3 tools/routine_inventory.py --write     # 重新生成
    python3 tools/routine_inventory.py --check     # 门禁（只读）
    python3 tools/routine_inventory.py --stats     # 只看统计
    python3 tools/routine_inventory.py --list unported [--bank engine]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC_REL = Path("refs/m1disasm/SRC")
SYM_REL = Path("refs/m1disasm/out/M1_NES_NTSC.sym")
OUT_REL = Path("data/status/routine_inventory.json")
OVR_REL = Path("data/status/routine_inventory_overrides.json")

_DATA_DIRECTIVES = (
    ".byte", ".byte", ".db", ".word", ".dw", ".ds", ".incbin",
    ".asc", ".asciitable", ".utf8", ".utf16",
)
_LABEL_RE = re.compile(r"^([A-Za-z_][\w@.{}]*)\s*:")

# 6502 助记符（wla-dx 方言）。用途：若 label 后第一行既不是数据指示、首 token 也不是
# 助记符（而是 `SetPaletteEntry …`/`SignMagSpeed …` 这类**生成数据的宏**），则按 data 计
# ——否则 palettes.asm / enemy_sprite_data 这类表会被误判成例程，虚增分母。
_MNEMONICS = frozenset("""
adc and asl bcc bcs beq bit bmi bne bpl brk bvc bvs clc cld cli clv cmp cpx cpy dec dex dey
eor inc inx iny jmp jsr lda ldx ldy lsr nop ora pha php pla plp rol ror rti rts sbc sec sed sei
sta stx sty tax tay tsx txa txs tya
""".split())

_LOCAL_LABEL = re.compile(r"^(?:RTS_|_|L[0-9A-Fa-f]{4}$)")
_AREA_SUFFIX = re.compile(r"_\{[A-Za-z0-9_]+\}$")
_LOCAL_SUFFIX = re.compile(r"^(.*)@(loop|loop_[A-Za-z0-9_]+|RTS|exitloop_[A-Za-z0-9_]+|[A-Za-z0-9_]+)$")
_RUST_FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?fn\s+([a-z0-9_]+)")


def snake_case(name: str) -> str:
    s = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name)
    s = re.sub(r"(?<=[A-Z])(?=[A-Z][a-z])", "_", s)
    return s.lower()


def rust_fn_index(root: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for path in sorted(root.glob("crates/**/src/**/*.rs")):
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for i, line in enumerate(text.splitlines(), 1):
            m = _RUST_FN_RE.match(line)
            if m:
                out.setdefault(m.group(1), f"{path.relative_to(root)}:{i}")
    return out


def parse_asm(root: Path) -> list[dict]:
    """源码级 label 清单：[{file,line,label,kind}]，kind ∈ {code,data}。"""
    rows: list[dict] = []
    for path in sorted((root / SRC_REL).rglob("*.asm")):
        if "/pal/" in path.as_posix():
            continue  # D6：保真期只做 NES_NTSC，PAL 变体不进分母
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        rel = str(path.relative_to(root))
        for i, raw in enumerate(lines):
            head = raw.split(";", 1)[0]
            m = _LABEL_RE.match(head.strip())
            if not m:
                continue
            label = _AREA_SUFFIX.sub("", m.group(1))
            if _LOCAL_LABEL.match(label):
                continue  # 汇编器自动名/局部出口标（RTS_Crawler06、L1234、_tmp）不算例程
            # 向后找第一条**既非空、也不是 label**的行：连续 label 链（多个标签指向
            # 同一块数据/代码）必须穿透，否则链尾的数据表会被误判成 code。
            j = i + 1
            nxt = ""
            while j < len(lines):
                head_j = lines[j].split(";", 1)[0].strip()
                if not head_j or head_j.startswith("@") or _LABEL_RE.match(head_j):
                    j += 1
                    continue
                nxt = head_j.lower()
                break
            first_tok = nxt[1:].split(None, 1)[0] if nxt.startswith(".") else (nxt.split(None, 1)[0] if nxt else "")
            kind = "data" if (nxt.startswith(_DATA_DIRECTIVES) or (first_tok and first_tok not in _MNEMONICS)) else "code"
            rows.append({"file": rel, "line": i + 1, "label": label, "kind": kind})
    return rows


def load_overrides(root: Path) -> dict[str, dict]:
    p = root / OVR_REL
    if not p.exists():
        return {}
    data = json.loads(p.read_text(encoding="utf-8"))
    return {str(k): dict(v) for k, v in (data.get("labels") or {}).items()}


def source_fingerprint(root: Path) -> str:
    h = hashlib.sha256()
    for path in sorted((root / SRC_REL).rglob("*.asm")):
        if "/pal/" in path.as_posix():
            continue  # 与 parse_asm 同口径（D6 NTSC-only）
        h.update(str(path.relative_to(root)).encode())
        h.update(path.read_bytes())
    return h.hexdigest()


def build(root: Path) -> dict:
    rows = parse_asm(root)
    fns = rust_fn_index(root)
    overrides = load_overrides(root)

    code: dict[str, list[dict]] = {}
    data_labels = 0
    seen: set[tuple[str, str]] = set()
    counts: Counter[str] = Counter()

    for row in rows:
        key = (row["file"], row["label"])
        if key in seen:
            continue
        seen.add(key)
        if row["kind"] == "data":
            data_labels += 1
            continue
        ovr = overrides.get(row["label"]) or {}
        status = str(ovr.get("status") or "")
        if status == "data":
            # 分析器把「label 后是 .if/.byte 组」的表误判成例程时，可在 overrides 里显式
            # 标 data 退回数据面（不计数、不算缺口）。必须给 note 说明是哪种表。
            data_labels += 1
            continue
        rust = str(ovr.get("rust") or "")
        if status not in ("ported", "candidate", "unported"):
            hit = fns.get(snake_case(row["label"]))
            status, rust = ("candidate", hit) if hit else ("unported", "")
        counts[status] += 1
        group = Path(row["file"]).stem
        code.setdefault(group, []).append(
            {"label": row["label"], "file": row["file"], "line": row["line"],
             "status": status, "rust": rust}
        )

    for entries in code.values():
        entries.sort(key=lambda e: (e["file"], e["line"]))

    return {
        "schema": 2,
        "source": str(SRC_REL),
        "source_fingerprint": source_fingerprint(root),
        "counts": {
            "code": sum(counts.values()),
            "ported": counts["ported"],
            "candidate": counts["candidate"],
            "unported": counts["unported"],
            "data_labels": data_labels,
        },
        "groups": {k: len(v) for k, v in sorted(code.items())},
        "entries": dict(sorted(code.items())),
    }


def write_inventory(root: Path, inv: dict) -> None:
    path = root / OUT_REL
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(inv, ensure_ascii=False, separators=(",", ":")) + "\n",
                    encoding="utf-8")


def check_inventory(root: Path) -> list[str]:
    path = root / OUT_REL
    if not path.exists():
        return [f"缺清单文件 {OUT_REL}（先跑 tools/routine_inventory.py --write）"]
    base = json.loads(path.read_text(encoding="utf-8"))
    cur = build(root)
    problems: list[str] = []

    if base.get("source_fingerprint") != cur["source_fingerprint"]:
        problems.append(
            "分母漂移：refs/m1disasm/SRC 指纹变了——重跑 --write 并在 commit 里说明"
            "（.sym/asm 是证据锚，改它必须解释）"
        )

    b_unp = int((base.get("counts") or {}).get("unported", 0))
    b_por = int((base.get("counts") or {}).get("ported", 0))
    if cur["counts"]["ported"] < b_por:
        return [
            f"ported 回退：{b_por} → {cur['counts']['ported']}"
            "（已复核登记的例程不得降级；确需降级须改 overrides 并在此说明）"
        ]
    if cur["counts"]["unported"] > b_unp:
        problems.append(
            f"unported 增长：{b_unp} → {cur['counts']['unported']}"
            "（缺口只减不增；要么补端口，要么在 overrides 登记为 data/out-of-scope）"
        )

    base_labels = {e["label"] for es in (base.get("entries") or {}).values() for e in es}
    cur_labels = {e["label"] for es in cur["entries"].values() for e in es}
    new_faces = sorted(cur_labels - base_labels)
    if new_faces:
        problems.append("清单外新例程：" + ", ".join(new_faces[:8]))
    return problems


def print_stats(inv: dict) -> None:
    c = inv["counts"]
    pct = (100.0 * c["candidate"] / c["code"]) if c["code"] else 0.0
    print(
        f"[清单] code 例程 {c['code']} 条：ported {c['ported']} / candidate {c['candidate']}"
        f"（{pct:.1f}%）/ unported {c['unported']}；data label {c['data_labels']} 条（不计缺口）"
    )
    for g, n in inv["groups"].items():
        es = inv["entries"][g]
        cnt = Counter(e["status"] for e in es)
        print(f"  {g:<22} {n:>4}  ported {cnt['ported']:>3}"
              f"  candidate {cnt['candidate']:>3}  unported {cnt['unported']:>4}")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="例程清单（结构驱动分母）")
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--stats", action="store_true")
    ap.add_argument("--list", choices=("ported", "candidate", "unported"))
    ap.add_argument("--group")
    ap.add_argument("--limit", type=int, default=40)
    args = ap.parse_args(argv)

    if args.write:
        inv = build(ROOT)
        write_inventory(ROOT, inv)
        print_stats(inv)
        print(f"[清单] 已写入 {OUT_REL}")
        return 0

    if args.check:
        problems = check_inventory(ROOT)
        for p in problems:
            print(f"[FAIL] {p}")
        if problems:
            return 1
        inv = build(ROOT)
        print(f"[PASS] routine-inventory：code {inv['counts']['code']}，"
              f"unported {inv['counts']['unported']}（无增长）、分母锚一致")
        return 0

    inv = build(ROOT)
    if args.list:
        shown = 0
        for g, es in inv["entries"].items():
            if args.group and g != args.group:
                continue
            for e in es:
                if e["status"] != args.list:
                    continue
                print(f"{e['status']:<9} {g:<20} {e['label']:<32} {e['rust']}")
                shown += 1
                if shown >= args.limit:
                    return 0
        return 0

    print_stats(inv)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
