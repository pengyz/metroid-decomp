-- tools/smoke_capture.lua —— S2 冒烟采集（Metroid headless 运行验证）
-- 用法: xvfb-run -a tools/mesen2/Mesen "rom/Metroid (USA).nes" tools/smoke_capture.lua /DoNotSaveSettings
-- 帧计数与 scenario_capture.lua 同约定：inputPolled 递增；endFrame 处截图。
local shotsDir = "/home/peng/workspace/metroid-decomp/shots/"
local frame = 0

local function log(s)
  local f = io.open("/tmp/mesen_smoke.log", "a")
  if f then f:write(s .. "\n") f:close() end
end

local function saveShot(tag)
  local ok, png = pcall(emu.takeScreenshot)
  if ok and png then
    local f = io.open(string.format("%s%s_f%05d.png", shotsDir, tag, frame), "wb")
    if f then f:write(png) f:close(); log("saved " .. tag .. " at frame " .. frame) end
  else
    log("takeScreenshot failed at frame " .. frame)
  end
end

emu.addEventCallback(function()
  frame = frame + 1
  local btn = {}
  -- 标题屏 push start → 密码屏，再按 A 以空密码开局（冒烟到可玩入口）
  if (frame >= 360 and frame <= 365) or (frame >= 480 and frame <= 485) then btn.start = true end
  if (frame >= 420 and frame <= 425) or (frame >= 540 and frame <= 545) then btn.a = true end
  pcall(function() emu.setInput(btn, 0) end)
end, "inputPolled")

emu.addEventCallback(function()
  if frame == 30 or frame == 150 or frame == 300 or frame == 400
     or frame == 500 or frame == 660 or frame == 900 or frame == 1200 then
    saveShot("smoke")
  end
  if frame >= 1200 then
    saveShot("final")
    log("smoke done at frame " .. frame)
    pcall(emu.stop, 0)
  end
end, "endFrame")

log("smoke script started")
