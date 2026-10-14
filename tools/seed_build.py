#!/usr/bin/env python3
# Assemble the normalized frozen disassembly seed from IDA bank exports (D3: projection, ROM is truth).
#
# Usage: python3 tools/seed_build.py
# Input : analysis/ida/bank{00..07}.json  (produced by tools/ida/run_all_banks.sh)
# Output: reference/seed/metroid.asm      (deterministic: byte-identical on re-run)
#
# Normalization rules (fixed for reproducibility):
#   - one line per IDA item: "bankNN:XXXX  <hex bytes>  <disasm or .byte ...>"
#   - data items emitted as up to 8 ".byte $XX" per line
#   - no timestamps / absolute paths in output
#
# Metroid (NES_NTSC)：8×16KB PRG，MMC1。bank0-6 → CPU $8000；bank7 → $C000 固定。
# 与 gunnac 版差异：8 banks × 16KB；无 far-call 列；窗口证据为 MMC1 语义。

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IDA_DIR = ROOT / "analysis" / "ida"
OUT = ROOT / "reference" / "seed" / "metroid.asm"
ROM = ROOT / "rom" / "Metroid (USA).nes"

# canonical CPU base per bank (bank7 fixed $C000; bank0-6 MMC1 window $8000)
BASES = {7: 0xC000}
WINDOW_EVIDENCE = {
    7: "MMC1 固定 bank（$C000-$FFFF，向量在 $FFFA；NMI=$C0D9 RESET=$FFB0 IRQ=$FFB0）",
    0: "MMC1 可切窗口（$8000）；bank0=Title/EndGame",
    1: "MMC1 可切窗口（$8000）；bank1=Brinstar",
    2: "MMC1 可切窗口（$8000）；bank2=Norfair",
    3: "MMC1 可切窗口（$8000）；bank3=Tourian",
    4: "MMC1 可切窗口（$8000）；bank4=Kraid",
    5: "MMC1 可切窗口（$8000）；bank5=Ridley",
    6: "MMC1 可切窗口（$8000）；bank6=Graphics",
}


def base_of(k: int) -> int:
    return BASES.get(k, 0x8000)


def main() -> None:
    rom_sha = hashlib.sha256(ROM.read_bytes()).hexdigest()
    lines = []
    lines.append("; =====================================================================")
    lines.append("; Metroid (USA).nes — normalized 6502 disassembly seed (FROZEN projection, D3)")
    lines.append("; =====================================================================")
    lines.append(f"; rom sha256: {rom_sha}")
    lines.append("; decoder   : IDA 9.1 processor M6502 (tools/ida/export_bank.py, linear sweep)")
    lines.append("; regen     : tools/ida/run_all_banks.sh && python3 tools/seed_build.py")
    lines.append("; freeze    : reference/seed/ 一经冻结不可变；重生成需 commit 说明（AGENTS.md §2）")
    lines.append("; windows   : bank0-6 -> $8000-$BFFF (MMC1 可切)  bank7 -> $C000-$FFFF (固定)")
    lines.append("; vectors   : NMI=$C0D9 RESET=$FFB0 IRQ=$FFB0 (bank7 fixed; file 0x1FFFA)")
    lines.append("; NOTE      : bankNN:XXXX = bank 内规范 CPU 地址；第三方命名（m1disasm 等）不在本轨")
    lines.append("")

    totals = {"items": 0, "funcs": 0, "xrefs": 0}
    covered = 0
    rom = ROM.read_bytes()
    prg = rom[0x10:0x20010]
    for k in range(8):
        data = json.loads((IDA_DIR / f"bank{k:02d}.json").read_text(encoding="utf-8"))
        assert data["bank"] == k and data["base"] == base_of(k), f"bank{k:02d} base mismatch"
        prg_off = k * 0x4000
        # A 级忠实性校验：items 字节重建 == ROM 对应 bank 切片
        rebuilt = b"".join(bytes.fromhex(i["bytes"]) for i in data["items"])
        assert rebuilt == prg[k * 0x4000:(k + 1) * 0x4000], f"bank{k:02d} items 字节与 ROM 不符"
        ev = WINDOW_EVIDENCE.get(k, "")
        lines.append(f"; ==== PRG bank {k:02d} | PRG 0x{prg_off:05X} | CPU ${base_of(k):04X}-"
                     f"${base_of(k) + 0x3FFF:04X} | {ev} ====")
        for item in data["items"]:
            ea = item["ea"]
            raw = bytes.fromhex(item["bytes"])
            covered += len(raw)
            if item["kind"] == "code":
                hx = " ".join(f"{b:02X}" for b in raw)
                lines.append(f"bank{k:02d}:{ea:04X}  {hx:<12s}  {item['text']}")
            else:
                for off in range(0, len(raw), 8):
                    chunk = raw[off:off + 8]
                    hx = " ".join(f"{b:02X}" for b in chunk)
                    lines.append(f"bank{k:02d}:{ea + off:04X}  {hx:<12s}  "
                                 + ",".join(f"${b:02X}" for b in chunk))
        totals["items"] += len(data["items"])
        totals["funcs"] += len(data["funcs"])
        totals["xrefs"] += len(data["xrefs"])
        lines.append("")

    assert covered == 0x20000, f"coverage {covered} != 131072"
    OUT.parent.mkdir(parents=True, exist_ok=True)
    # newline="\n"：seed 冻结基准按 LF 字节面哈希，禁 Windows CRLF 翻译
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    n_lines = sum(1 for _ in open(OUT, encoding="utf-8"))
    print(f"wrote {OUT.relative_to(ROOT)} lines={n_lines} "
          f"items={totals['items']} funcs={totals['funcs']} xrefs={totals['xrefs']} "
          f"covered={covered}")


if __name__ == "__main__":
    main()
