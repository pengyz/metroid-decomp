-- tools/probe_savestate_load.lua —— task-32 首件：Mesen2 savestate 装载能力探针（v2）
-- v1 发现（2026-10-16）：API 词族实锤——emu.createSavestate/loadSavestate/setState/
-- getState（**无** saveStateFile/loadStateSlot——v1 猜名全错，枚举面救场同 r3 教训）。
-- v2：①真 API 签名地毯（错误串全文）②getState/setState 内存往返（夹具原语本体）
--     ③createSavestate/loadSavestate 文件往返 ④事件回调上下文装载（fixture 触发路径）
-- 用法: xvfb-run -a tools/mesen2/Mesen "rom/Metroid (USA).nes" tools/probe_savestate_load.lua /DoNotSaveSettings
local dir = "/home/peng/workspace/metroid-decomp/analysis/"
local logF = io.open(dir .. "probe_savestate_load.log", "wb")
local function log(s) logF:write(s .. "\n") logF:flush() end
local MT = emu.memType and emu.memType.cpu or nil
log("memType.cpu = " .. tostring(MT))

local function rb(a)
  local ok, v = pcall(emu.read, a, MT)
  return ok and v or -1
end
local function wb(a, v)
  local ok, err = pcall(emu.write, a, v, MT)
  return ok, err
end

-- ① 真 API 签名地毯（v1 已证 getState 全机状态表；v2 实录：无参 createSavestate
-- **阻塞**（疑原生对话框，超时 124）——v3 带参试探殿后，setState 往返先行）
log("== ① setState 签名（getState 孪生）==")
local snap
local ok, err = pcall(function() snap = emu.getState() end)
log(string.format("  getState() → ok=%s keys=%s", tostring(ok),
  ok and type(snap) == "table" and "table(全机状态)" or type(snap)))

-- ② getState/setState 内存往返（夹具原语本体）
log("== ② getState/setState 内存往返 ==")
local MARK = 0x0700
local plan = { phase = 0 }
local frame = 0
local snap2
local MT = 8  -- v4：MEMTYPE=数值枚举 8（scenario_capture 同源——emu.memType.cpu=nil 实录）
local function rb(a)
  local o, v = pcall(emu.read, a, MT)
  return o and v or -1
end
local function wb(a, v)
  return pcall(emu.write, a, v, MT)
end
emu.addEventCallback(function()
  frame = frame + 1
  if plan.phase == 0 and frame == 20 then
    wb(MARK, 0xAA)
    snap2 = emu.getState()
    log(string.format("② f20 快照后: 标记=$%02X（期望 AA）", rb(MARK)))
    wb(MARK, 0x55)
    log(string.format("② f20 覆写后: 标记=$%02X（期望 55）", rb(MARK)))
    local oks, e2 = pcall(emu.setState, snap2)
    log(string.format("② f20 setState: ok=%s err=%s", tostring(oks), tostring(e2)))
    log(string.format("② f20 setState 后: 标记=$%02X（期望 AA=内存往返成功）", rb(MARK)))
    plan.phase = 1
  elseif plan.phase == 1 and frame == 30 then
    -- ③ exec 回调上下文 savestate（v3 实录：须在主 CPU exec 内存回调内调用）——
    -- 注册一次性 exec 回调 @C0D3，回调内完成 createSavestate→覆写→loadSavestate 往返
    local done = false
    local okreg = false
    if emu.callbackType and emu.callbackType.exec then
      okreg = pcall(emu.addMemoryCallback, function()
        if done then return end
        done = true
        local oka, a = pcall(emu.createSavestate)
        log(string.format("③ exec 内 createSavestate(): ok=%s ret_type=%s val=%s",
          tostring(oka), type(a), tostring(a)))
        wb(MARK, 0x33)
        log(string.format("③ exec 内覆写: 标记=$%02X（期望 33）", rb(MARK)))
        if oka then
          local okl, e3 = pcall(emu.loadSavestate, a)
          log(string.format("③ exec 内 loadSavestate(ret): ok=%s err=%s", tostring(okl), tostring(e3)))
          log(string.format("③ exec 内装载后: 标记=$%02X（期望快照值）", rb(MARK)))
        end
      end, emu.callbackType.exec, 0xC0D3, 0xC0D3)
    end
    log(string.format("③ f30 exec 回调注册: %s", tostring(okreg)))
    plan.phase = 2
  elseif plan.phase == 2 and frame == 35 then
    log(string.format("③ f35 连续性: 帧=%d 标记=$%02X", frame, rb(MARK)))
    plan.phase = 3
  elseif plan.phase == 3 and frame == 40 then
    log("== 探针完成 ==")
    logF:close()
    emu.stop()
  end
end, "endFrame")
log("③ 事件回调已注册（帧 20/30/35/40）")
