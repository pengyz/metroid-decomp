-- tools/probe_dispatch_diag.lua —— r3 环⑤证伪探针 v7：枚举正牌语义 + 错误串
-- v6 发现：callbackType.exec=2（正牌枚举）、counterType.execCount（执行计数）、
-- getCdlData/getAccessCounters 存在但签名不明（pcall 失败，错误串未打印）。
-- 本探针：①错误串全文；②多种签名地毯式 pcall；③**字符串 "execute" vs 枚举 2 对照**：
--   C0BF（每帧确证执行的 JSR opcode）在两种注册下的命中数——若枚举 2 有命中=opcode
--   fetch 可见，分发探针与覆盖根治同时解锁。
-- 用法: xvfb-run -a tools/mesen2/Mesen "rom/Metroid (USA).nes" tools/probe_dispatch_diag.lua /DoNotSaveSettings
local dir = "/home/peng/workspace/metroid-decomp/analysis/"
local logF = io.open(dir .. "probe_dispatch_diag7.log", "wb")
local function log(s) logF:write(s .. "\n") logF:flush() end

local counts = {str = {}, enum = {}}

-- ③ 对照注册：C0BF/C0C0/C0C1/C0D9/C27C 各两路
local addrs = {0xC0BF, 0xC0C0, 0xC0C1, 0xC0D9, 0xC27C}
for _, a in ipairs(addrs) do
  counts.str[a], counts.enum[a] = 0, 0
  local ok1 = pcall(emu.addMemoryCallback, function(address) counts.str[address] = counts.str[address] + 1 end,
    "execute", a, a)
  local ok2 = pcall(emu.addMemoryCallback, function(address) counts.enum[address] = counts.enum[address] + 1 end,
    emu.callbackType.exec, a, a)
  log(string.format("reg %04X str=%s enum=%s", a, tostring(ok1), tostring(ok2)))
end

-- ①② 签名地毯式（打印错误串全文）
local probes = {
  {"getCdlData()", function() return emu.getCdlData() end},
  {"getCdlData(0,16)", function() return emu.getCdlData(0, 16) end},
  {"getCdlData(0x8000,16)", function() return emu.getCdlData(0x8000, 16) end},
  {"getCdlData(0x8000,16,8)", function() return emu.getCdlData(0x8000, 16, 8) end},
  {"getCdlData(0,16,45)", function() return emu.getCdlData(0, 16, 45) end},
  {"getCdlData(0x8000,0x800F,8)", function() return emu.getCdlData(0x8000, 0x800F, 8) end},
  {"getAccessCounters(0x8000,0x800F)", function() return emu.getAccessCounters(0x8000, 0x800F) end},
  {"getAccessCounters(0x8000,0x800F,2)", function() return emu.getAccessCounters(0x8000, 0x800F, 2) end},
  {"getAccessCounters(0x8000,0x800F,2,8)", function() return emu.getAccessCounters(0x8000, 0x800F, 2, 8) end},
  {"getAccessCounters(0x8000,16,2,8)", function() return emu.getAccessCounters(0x8000, 16, 2, 8) end},
}
for _, p in ipairs(probes) do
  local ok, res = pcall(p[2])
  if ok then
    local t = type(res)
    local extra = ""
    if t == "string" then
      extra = " len=" .. #res .. " head=" .. res:sub(1, 24):gsub(".", function(c) return string.format("%02X", string.byte(c)) end)
    elseif t == "table" then
      local n = 0
      for _ in pairs(res) do n = n + 1 end
      extra = " n=" .. n
      local i = 0
      for k, v in pairs(res) do
        i = i + 1
        if i <= 6 then extra = extra .. string.format(" [%s]=%s", tostring(k), tostring(v)) end
      end
    elseif t == "number" then
      extra = " =" .. tostring(res)
    end
    log(p[1] .. " OK " .. t .. extra)
  else
    log(p[1] .. " ERR " .. tostring(res))
  end
end

local frame = 0
emu.addEventCallback(function()
  frame = frame + 1
  if frame == 60 then
    for _, a in ipairs(addrs) do
      log(string.format("count %04X: str=%d enum=%d", a, counts.str[a], counts.enum[a]))
    end
    logF:close()
    pcall(emu.stop)
  end
end, "endFrame")
