-- AccuracyCoin oracle evidence dump for Mesen2 (metroid-decomp tools/oracle/accuracycoin)
-- Reads CPU RAM (memType=8=NesMemory), PPU nametable + CHR-RAM (memType=9=NesPpuMemory).
-- !! Mesen2 Lua emu.read signature is read(returnSigned, memType, address) — memType MUST be
--    passed explicitly; the default (0=SnesMemory) returns 0 for everything.
-- Screen text decoding: tiles 0x00-09=digits 0-9, 0x0A-1F=A-V, 0x24=space, 0x40-5F=red A-Z glyphs.
-- Result vector (v3.6.2): $0037=PostAllTestTally, $0038=PostAllPassTally, slots $0400-$0498
--   (result_* table in AccuracyCoin.asm:144-321; bit0=1 PASS, bit0=0 FAIL err=(byte>>2)).
--
-- IMPORTANT (r9 复核更正): the battery does NOT run at boot — it only starts after a
-- Start press on the menu (AccuracyCoin.asm:18183-18186 `AND #$10 ; Start …
-- JSR AutomaticallyRunEveryTestInROM`). This script therefore injects Start once.
-- OUT path must be ABSOLUTE: Mesen2 chdirs into its config dir at startup, so
-- relative io.open paths land in ~/.config/Mesen2/ (实测). run_accuracycoin.py
-- substitutes {{OUT}} with the absolute target path before launching.
local f = 0
local OUT = "{{OUT}}"
local out = io.open(OUT, "w")
out:write("lua loaded\n"); out:flush()
local function dump()
  out:write(string.format("== f=%d ==\n", f))
  out:write("CHR:\n")
  for t = 0, 127 do
    out:write(string.format("T%02X:", t))
    for b = 0, 15 do out:write(string.format("%02X", emu.read(t*16 + b, 9) or 0)) end
    out:write("\n")
  end
  out:write("NT:\n")
  for r = 0, 29 do
    for c = 0, 31 do out:write(string.format("%02X", emu.read(0x2000 + r*32 + c, 9) or 0)) end
    out:write("\n")
  end
  out:write("RAM:\n")
  for i = 0, 2047 do
    out:write(string.format("%02X", emu.read(i, 8) or 0))
    if (i % 32) == 31 then out:write("\n") end
  end
  out:flush()
end
emu.addEventCallback(function()
  -- Start injection during the inputPolled phase (f=60-90 window, 8 frames) so the
  -- menu launches the full battery (see header comment).
  if f >= 60 and f <= 90 then emu.setInput({start = (f % 2) == 0}, 0) end
  if f > 90 and f <= 95 then emu.setInput({start = false}, 0) end
end, "inputPolled")
emu.addEventCallback(function()
  f = f + 1
  if f % 600 == 0 then out:write(string.format("tick f=%d\n", f)); out:flush() end
  -- Dump after the battery completes and periodically after.
  if f >= 600 and (f - 600) % 1800 == 0 then dump() end
  if f == 16000 then dump(); out:write("ENDDUMP\n"); out:flush(); out:close() end
end, "endFrame")
