local out=io.open("/tmp/fceux222_ref.log","w")
out:write("# f hp_lo hp_hi status gm\n")
local f=0
while true do
  out:write(string.format("%d %02x %02x %02x %02x\n", f,
    memory.readbyte(0x0106), memory.readbyte(0x0107),
    memory.readbyte(0x0300), memory.readbyte(0x0013)))
  if (f%500)==0 then out:flush() end
  f=f+1
  if f>=4300 then out:flush(); out:close(); os.exit(0) end
  emu.frameadvance()
end
