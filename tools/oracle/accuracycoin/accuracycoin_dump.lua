-- AccuracyCoin oracle evidence dump for Mesen2 (metroid-decomp tools/oracle/accuracycoin)
-- Reads CPU RAM (memType=8=NesMemory), PPU nametable + CHR-RAM (memType=9=NesPpuMemory).
-- !! Mesen2 Lua emu.read signature is read(returnSigned, memType, address) — memType MUST be
--    passed explicitly; the default (0=SnesMemory) returns 0 for everything.
-- Screen text decoding: tiles 0x00-09=digits 0-9, 0x0A-1F=A-V, 0x24=space, 0x40-5F=red A-Z glyphs.
-- Result vector (v3.6.2): $0037=PostAllTestTally, $0038=PostAllPassTally, slots $0400-$0498
--   (result_* table in AccuracyCoin.asm:144-321; bit0=1 PASS, bit0=0 FAIL err=(byte>>2)).
local f = 0
local OUT = (arg and arg[1]) or "accuracycoin_dump.log"
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
  f = f + 1
  if f % 600 == 0 then out:write(string.format("tick f=%d\n", f)); out:flush() end
  -- AccuracyCoin auto-runs every test at boot (AutomaticallyRunEveryTestInROM); Start just
  -- dismisses the title/menu. Dump after the battery completes and periodically after.
  if f >= 600 and (f - 600) % 1800 == 0 then dump() end
  if f == 16000 then dump(); out:write("ENDDUMP\n"); out:flush(); out:close() end
end, "endFrame")
