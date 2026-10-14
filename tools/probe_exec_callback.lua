-- tools/probe_exec_callback.lua —— F1 根因微探针：判定 Mesen2 addMemoryCallback 的真实语义
-- 用法: xvfb-run -a tools/mesen2/Mesen "rom/Metroid (USA).nes" tools/probe_exec_callback.lua /DoNotSaveSettings
-- 同帧注册两路回调并各自记账，同时逐帧采样 cpu.pc（确证执行过的地址）做不变量对照：
--   A 路: 字符串 "execute"          （当前 scenario_capture.lua 的用法——被复核 F1 证伪）
--   B 路: emu.memCallbackType.execute 枚举
-- 判据: 每帧 cpu.pc ∈ 该路集合（NMI/主循环每帧都在跑，300 帧样本足以暴露漏记）
--       且该路集合 ⊆ 指令入口集（与 seed 行首对照由 tools/ 分析侧做）
local dir = "/home/peng/workspace/metroid-decomp/analysis/"
local frame = 0
local setA, setB, pcSet = {}, {}, {}
local enumDump = "nil"

if emu.memCallbackType then
  local parts = {}
  for k, v in pairs(emu.memCallbackType) do parts[#parts + 1] = tostring(k) .. "=" .. tostring(v) end
  table.sort(parts)
  enumDump = table.concat(parts, ",")
end

local okA = pcall(emu.addMemoryCallback, function(address) setA[address] = true end,
  "execute", 0x8000, 0xFFFF)
local okB = false
if emu.memCallbackType and emu.memCallbackType.execute then
  okB = pcall(emu.addMemoryCallback, function(address) setB[address] = true end,
    emu.memCallbackType.execute, 0x8000, 0xFFFF)
end

emu.addEventCallback(function()
  frame = frame + 1
  -- getState 返回平铺键（scenario_capture.lua cpuJson 实证：st["cpu.pc"]）；双键都取防漏
  local ok, st = pcall(emu.getState)
  if ok and st then
    local pc = st["cpu.pc"] or (st.cpu and st.cpu.pc)
    if pc then pcSet[pc] = true end
  end
end, "inputPolled")
emu.addEventCallback(function()
  local ok, st = pcall(emu.getState)
  if ok and st then
    local pc = st["cpu.pc"] or (st.cpu and st.cpu.pc)
    if pc then pcSet[pc] = true end
  end
  if frame >= 300 then
    local function dump(path, set, mode)
      local f = io.open(path, "w")
      if not f then return end
      local n = 0
      local addrs = {}
      for a, _ in pairs(set) do addrs[#addrs + 1] = a end
      table.sort(addrs)
      f:write(string.format('{"mode":"%s","count":%d}\n', mode, #addrs))
      for _, a in ipairs(addrs) do f:write(string.format('{"a":"%04X"}\n', a)) end
      f:close()
    end
    dump(dir .. "probeA.jsonl", setA, okA and "string-execute" or "regfail")
    dump(dir .. "probeB.jsonl", setB, okB and "enum-execute" or "regfail")
    dump(dir .. "probePC.jsonl", pcSet, "cpu-pc-samples")
    local f = io.open(dir .. "probe_meta.jsonl", "w")
    if f then
      f:write(string.format('{"frames":%d,"regA":%s,"regB":%s,"enum":"%s"}\n',
        frame, tostring(okA), tostring(okB), enumDump))
      f:close()
    end
    pcall(emu.stop)
  end
end, "endFrame")
