-- tools/movie_ref/bizhawk_ref.lua —— BizHawk(quickerNES) 逐帧 RAM 记录（取证用）
--
-- 用途：加载 .bk2 录像后逐帧写出 RAM 观测面 + 录像输入面，供与 Rust 重制逐帧比对。
-- 字段与地址来源（全部为仓库内锚，禁止猜）：
--   $0106 HealthLo / $0107 HealthHi —— crates/game-core/src/ram_map.rs:169 SAMUS_HEALTH
--                                      （16bit，BCD：$0106=低两位、$0107=高位）
--   $0300 SamusStatus               —— crates/game-core/src/ram_map.rs:223 SAMUS_STATUS
--   $005A RoomNumber                —— crates/game-core/src/ram_map.rs:216 ROOM_NUMBER（$FF=undefined）
--   $0049 ScrollDir                 —— crates/game-core/src/ram_map.rs:59 SCROLL_DIR
--   $0014/$0015 Joy1/2Status        —— crates/game-core/src/ram_map.rs:70,73（ROM 读手柄后的锁存）
-- 手柄位形 A=bit7 … Right=bit0（hardware.asm:89-96 / tools/scenario_run.py BUTTON_BITS）。
-- p1/p2 直接取 `movie.getinput(f)`（BizHawk 2.10 Lua API：loaded movie 的第 f 帧输入）；
-- joy*_latch 取 $14/$15（ROM 侧实际消费面）——两者互为「输入确已喂入」的证据。
--
-- 用法（BizHawk 启动器会 cd 到 /tmp/bh，故输出用绝对路径）：
--   ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy xvfb-run -a /tmp/bh/EmuHawkMono.sh \
--     --chromeless --lua tools/movie_ref/bizhawk_ref.lua \
--     --movie <movie>.bk2 "rom/Metroid (USA).nes"

-- 输出路径：默认 analysis/traces/bizhawk-6725M.jsonl；用环境变量 BH_OUT 覆盖为其它绝对路径。
-- （BizHawk 启动器会 cd 到 /tmp/bh，相对路径会落错地方 ⇒ 这里只接受绝对路径。）
local function env_str(name, dflt)
  local ok, v = pcall(function() return os.getenv(name) end)
  if ok and type(v) == "string" and v ~= "" then return v end
  return dflt
end
local OUT_PATH = env_str("BH_OUT",
  "/home/peng/workspace/metroid-decomp/analysis/traces/bizhawk-6725M.jsonl")
-- 帧数：默认 1200 ⇒ 导出 f=0..1199 共 1200 行（够前 1000 帧比对 + 对齐余量）；
-- 全片导出（录像 28408 帧）用环境变量 `BH_FRAMES=28408` 覆盖，不必改脚本。
-- 【2026-11-18 修正】旧实现是 `for frame = 0, TOTAL_FRAMES`（**闭区间**）⇒ BH_FRAMES=N 实际
-- 导出 N+1 行；当 N 等于录像帧数（28408）时会去迭代越界帧 28408，Lua 报错触发模态窗 ⇒
-- 无头进程僵死（实测 STAT Sl、CPU 53.9%、输出文件停止增长），且 io.write 只每 100 帧 flush
-- 一次 ⇒ 末 7 行留在缓冲里丢失（28401 行 / 期望 28408）。现改半开区间：BH_FRAMES=N ⇒ f=0..N-1。
-- 注意：`--userdata k:v` + `client.getuserdata` 在本构建实测会挂死（见 findings），故用 os.getenv。
local function env_num(name, dflt)
  local ok, v = pcall(function() return os.getenv(name) end)
  if ok and type(v) == "string" and tonumber(v) then return tonumber(v) end
  return dflt
end
local TOTAL_FRAMES = env_num("BH_FRAMES", 1200)

local BIT = {
  A = 0x80, B = 0x40, Select = 0x20, Start = 0x10,
  Up = 0x08, Down = 0x04, Left = 0x02, Right = 0x01,
}

-- movie.getinput(f) → {["P1 Start"]=true, ...}：按前缀取端口，按按钮名置位。
local function movie_pad(f, port)
  local ok, t = pcall(movie.getinput, f)
  if not ok or type(t) ~= "table" then return -1 end
  local prefix = "P" .. tostring(port) .. " "
  local v = 0
  for k, val in pairs(t) do
    if val and type(k) == "string" and k:sub(1, #prefix) == prefix then
      local b = BIT[k:sub(#prefix + 1)]
      if b then v = v + b end
    end
  end
  return v
end

-- 【2026-11-18 相位修正】旧实现在 `emu.frameadvance()` **之前**读 RAM ⇒ 行 f 记的是
-- 「frame f 开工前」的状态（= frame f-1 的收工态），而同行的 `p1` 取 `movie.getinput(f)`（frame f 的输入）
-- ⇒ 轨迹文件**内部**状态与输入错一行。三方实测（f40..1200，四字段）：biz[f] ↔ Mesen2[f] 1061/1160、
-- biz[f] ↔ Mesen2[f±1] 更差（1053/1053）、而 Rust[f] ↔ Mesen2[f] 1133/1153 ⇒ 错位在**记录器**不在 Rust。
-- 现在改为「先 frameadvance 再读」：行 f = frame f 的**收工态**，与同行 `p1`=input(f) 同相位。
local fh = assert(io.open(OUT_PATH, "w"))
local n = 0
for frame = 0, TOTAL_FRAMES - 1 do
  emu.frameadvance()
  local lo = memory.read_u8(0x0106, "RAM")
  local hi = memory.read_u8(0x0107, "RAM")
  local st = memory.read_u8(0x0300, "RAM")
  local rm = memory.read_u8(0x005A, "RAM")
  local sd = memory.read_u8(0x0049, "RAM")
  local j1 = memory.read_u8(0x0014, "RAM")
  local j2 = memory.read_u8(0x0015, "RAM")
  local p1 = movie_pad(frame, 1)
  local p2 = movie_pad(frame, 2)
  fh:write(string.format(
    '{"f":%d,"health_lo":%d,"health_hi":%d,"status":%d,"room":%d,"scrolldir":%d,"p1":%d,"p2":%d,"joy1_latch":%d,"joy2_latch":%d}\n',
    frame, lo, hi, st, rm, sd, p1, p2, j1, j2))
  n = n + 1
  if (frame % 100) == 0 then fh:flush() end
end
fh:flush()
fh:close()
console.log(string.format("bizhawk_ref: wrote %d rows to %s (framecount=%d)", n, OUT_PATH, emu.framecount()))
client.exit()
