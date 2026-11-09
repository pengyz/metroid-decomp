-- Metroid WRAM $0600-$06FF dump harness (metroid-decomp tools/oracle, audio RAM contract evidence)
-- Adapted from accuracycoin_dump.lua (same MemType semantics: 8=NesMemory).
-- Path: title screen -> press Start (f=60-90 window) -> game starts -> sound engine active.
-- Dumps $0600-$06FF at f=600,900,1200,1500; ENDDUMP at f=1600.
local f = 0
local OUT = "{{OUT}}"
local out = io.open(OUT, "w")
out:write("lua loaded\n"); out:flush()
local function dump(tag)
  out:write(string.format("== %s f=%d ==\n", tag, f))
  for base = 0x600, 0x6FF, 16 do
    out:write(string.format("%03X:", base))
    for i = 0, 15 do out:write(string.format(" %02X", emu.read(base + i, 8) or 0)) end
    out:write("\n")
  end
  out:flush()
end
emu.addEventCallback(function()
  if f >= 60 and f <= 90 then emu.setInput({start = (f % 2) == 0}, 0) end
  if f > 90 and f <= 95 then emu.setInput({start = false}, 0) end
end, "inputPolled")
emu.addEventCallback(function()
  f = f + 1
  if f == 600 then dump("t600") end
  if f == 900 then dump("t900") end
  if f == 1200 then dump("t1200") end
  if f == 1500 then dump("t1500") end
  if f == 1600 then out:write("ENDDUMP\n"); out:flush(); out:close() end
end, "endFrame")
