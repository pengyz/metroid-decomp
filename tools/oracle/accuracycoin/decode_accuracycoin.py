#!/usr/bin/env python3
"""AccuracyCoin per-test result decoder (metroid-decomp tools/oracle/accuracycoin).

Reads a dump log produced by accuracycoin_dump.lua (RAM section, 2048 bytes from $0000)
and the upstream AccuracyCoin.asm result_* slot table, and prints the per-test
PASS/FAIL verdict table plus the pass tally cross-check against $0037/$0038.

Slot semantics (AccuracyCoin.asm ~:1000-1015): result byte AND #01 nonzero = PASS
(tally INC); bit0=0 = FAIL with error code (byte>>2). Multi-byte slots: result_CPUInstr
occupies $0401-$0402. $0421 must stay $00 (SRE_47 quirk, asm:159). DRAW-page results
live at $03FF/$0300-page and are not counted in the all-test tally.

Usage: python3 decode_accuracycoin.py <dump.log> [accuracycoin.asm]
"""
import re
import sys

DEFAULT_ASM = "https://raw.githubusercontent.com/100thCoin/AccuracyCoin/master/AccuracyCoin.asm"
SKIP_SLOTS = {0x402, 0x421}  # CPUInstr extension byte; SRE_47 quirk sentinel


def load_slots(asm_path):
    src = open(asm_path).read()
    slots = {}
    for m in re.finditer(r"^result_(\w+)\s*=\s*\$(\w+)", src, re.M):
        slots[int(m.group(2), 16)] = m.group(1)
    return slots


def load_ram(dump_path, frame=600):
    txt = open(dump_path).read()
    pat = r"== f=%d ==.*?RAM:\n((?:[0-9A-F]{64}\n){64})" % frame
    fm = re.search(pat, txt, re.S)
    if fm is None:  # fall back to first RAM section
        fm = re.search(r"RAM:\n((?:[0-9A-F]{64}\n){64})", txt, re.S)
    if fm is None:
        sys.exit("no RAM section found in dump")
    return bytes.fromhex("".join(l.strip() for l in fm.group(1).split()))


def main():
    dump_path = sys.argv[1]
    asm_path = sys.argv[2] if len(sys.argv) > 2 else None
    ram = load_ram(dump_path)
    print(f"$0037 TestTally={ram[0x37]}  $0038 PassTally={ram[0x38]}")
    if asm_path is None:
        sys.exit("pass the path to AccuracyCoin.asm (download: " + DEFAULT_ASM + ")")
    slots = load_slots(asm_path)
    rows = []
    for a, n in sorted(slots.items()):
        if not (0x400 <= a <= 0x498) or n in ("Unimplemented",) or a in SKIP_SLOTS:
            continue
        v = ram[a]
        if n == "SRE_47":
            rows.append((a, n, "QUIRK-OK" if v == 0 else "QUIRK-VIOLATION", v))
            continue
        status = "PASS" if v & 1 else ("FAIL" if v else "NOT-RUN")
        rows.append((a, n, status, v))
    fails = [r for r in rows if r[2] == "FAIL"]
    passes = [r for r in rows if r[2] == "PASS"]
    print(f"tests={len([r for r in rows if r[2] != 'QUIRK-OK'])} "
          f"pass={len(passes)} fail={len(fails)}")
    for a, n, s, v in fails:
        print(f"  {a:04X} {n} v={v:02X} err={(v>>2):02X}h")
    print("--- PASS ---")
    print(" ".join(n for _, n, s, v in passes))


if __name__ == "__main__":
    main()
