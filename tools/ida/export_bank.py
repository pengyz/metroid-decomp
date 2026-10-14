#!/usr/bin/env python3
# IDAPython batch export (runs INSIDE idat). Projection generator only; ROM bytes are truth (D3).
#
# Usage (idat batch):
#   S1_BANK=0 S1_BASE=0x8000 S1_OUT=analysis/ida/bank00.json \
#     idat -A -pm6502 -S tools/ida/export_bank.py -L analysis/ida/bank00.log analysis/ida/bank00.bin
#
# Emits deterministic JSON per PRG bank (Metroid: 8 banks × 16KB, MMC1):
#   items[]   every byte covered: {ea, size, kind: code|data, bytes, text}
#   xrefs[]   {from, to, kind: call|jump|ref} for in-bank address operands
#   funcs[]   subroutine ranges from mechanical entry starts (JSR/JMP targets + segment start)
# Mechanical only: no semantic naming (pipeline §7 机械层).
#
# 与 gunnac 版差异：BANK_SIZE=0x4000；无 far-call trampoline 约定（Gun-Nac 特有 JSR $E649）；
# Metroid 跨 bank 调用经 MMC1 写序 + $80xx 间接入口，属 S3 考证对象，此处不预设。

import json
import os

import ida_auto
import ida_bytes
import ida_funcs
import ida_idp
import ida_segment
import ida_ua
import idaapi
import idc

BANK = int(os.environ["S1_BANK"])
BASE = int(os.environ["S1_BASE"], 16)
OUT = os.environ["S1_OUT"]
BANK_SIZE = 0x4000
END = BASE + BANK_SIZE

ida_idp.set_processor_type("M6502", ida_idp.SETPROC_LOADER)

seg = ida_segment.get_first_seg()
if seg is None:
    raise SystemExit("no segment loaded")
moved = ida_segment.move_segm(seg, BASE, ida_segment.MSF_SILENT)
if not moved:
    # keep going only if already at BASE (idempotent rerun)
    if seg.start_ea != BASE:
        raise SystemExit("move_segm to 0x%X failed (rc=%r)" % (BASE, moved))
ida_segment.set_segm_name(seg, "PRG_BANK%02d" % BANK)
ida_auto.auto_wait()

# ---- deterministic linear sweep: cover every byte as code or data ----
ea = BASE
while ea < END:
    if ida_bytes.is_head(ida_bytes.get_flags(ea)):
        nxt = ida_bytes.next_head(ea, END)
        ea = nxt if nxt and nxt != idaapi.BADADDR else END
        continue
    n = ida_ua.create_insn(ea)
    if n <= 0:
        ida_bytes.create_data(ea, ida_bytes.FF_BYTE, 1, 0)
        ea += 1
    else:
        ea += n
ida_auto.auto_wait()

# ---- mechanical entry starts: JSR targets + JMP(abs) targets + segment base + ROM vectors ----
# S1_VECTORS（可选，驱动从 ROM 尾部向量表读出）：逗号分隔 0xADDR；落本 bank 范围者为 entry。
# 依据：向量指向的入口是机器可读事实（Metroid: NMI=$C0D9 RESET=$FFB0 IRQ=$FFB0 @ bank7）。
entry_starts = {BASE}
for tok in os.environ.get("S1_VECTORS", "").split(","):
    tok = tok.strip()
    if tok:
        v = int(tok, 16)
        if BASE <= v < END:
            entry_starts.add(v)
xrefs = []
for ea in range(BASE, END):
    if not ida_bytes.is_code(ida_bytes.get_flags(ea)):
        continue
    insn = ida_ua.insn_t()
    if ida_ua.decode_insn(insn, ea) <= 0:
        continue
    mnem = idc.print_insn_mnem(ea)
    for op in insn.ops:
        if op.type in (ida_ua.o_near, ida_ua.o_far, ida_ua.o_mem):
            tgt = op.addr
            if BASE <= tgt < END:
                kind = "call" if mnem == "JSR" else ("jump" if mnem in ("JMP", "JMPA") else "ref")
                xrefs.append({"from": ea, "to": tgt, "kind": kind})
                if kind in ("call", "jump"):
                    entry_starts.add(tgt)

# ---- create functions at mechanical entries; IDA finds RTS/RTI boundaries ----
for s in sorted(entry_starts):
    ida_funcs.add_func(s)
ida_auto.auto_wait()
funcs = []
for s in sorted(entry_starts):
    f = ida_funcs.get_func(s)
    if f:
        funcs.append({"start": f.start_ea, "end": f.end_ea, "size": f.end_ea - f.start_ea})

# ---- dump every byte: instructions + data items ----
items = []
ea = BASE
while ea < END:
    flags = ida_bytes.get_flags(ea)
    size = ida_bytes.get_item_size(ea) or 1
    if ida_bytes.is_code(flags):
        text = idc.generate_disasm_line(ea, 0) or ""
        items.append({
            "ea": ea,
            "size": size,
            "kind": "code",
            "bytes": ida_bytes.get_bytes(ea, size).hex(),
            "text": text.replace("\t", " "),
        })
    else:
        items.append({
            "ea": ea,
            "size": size,
            "kind": "data",
            "bytes": ida_bytes.get_bytes(ea, size).hex(),
            "text": "",
        })
    ea += size

out = {
    "bank": BANK,
    "base": BASE,
    "size": BANK_SIZE,
    "ida_version": idaapi.get_kernel_version(),
    "processor": "m6502",
    "items": items,
    "xrefs": xrefs,
    "funcs": funcs,
}
with open(OUT, "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False)
print("S1 export bank%02d base=0x%04X items=%d funcs=%d xrefs=%d -> %s"
      % (BANK, BASE, len(items), len(funcs), len(xrefs), OUT))
idc.qexit(0)
