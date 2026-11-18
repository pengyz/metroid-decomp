-- tools/movie_ref/bizhawk_shots.lua —— BizHawk 逐帧像素面取证（L4 对拍 emu 腿）。
--
-- 用途：与 tools/movie_ref/bizhawk_ref.lua（RAM 轨迹腿）互补——本脚本只采像素面。
-- 采法：`client.screenshot(path)`（BizHawk 2.10 Lua API，实测可写 256x224 PNG，
--       含 sprite 与 HUD，见 docs/findings/movie-bizhawk-6725m.md §10）。
--
-- 环境变量：
--   BH_SHOT_DIR    必填，PNG 落盘目录（不存在则创建失败，由外壳 mkdir -p）
--   BH_SHOT_FRAMES 必填，逗号分隔帧号（`20,30,940`）；帧号 = emu 帧（0 起）
--   BH_SHOT_END    选填，模拟到第几帧结束（缺省 = BH_SHOT_FRAMES 最大值）
--
-- 输出：<BH_SHOT_DIR>/biz_f<NNNNN>.png（帧号 5 位补零）
-- 退出：跑完 BH_SHOT_END 后 client.exit()；进程自杀（SIGKILL）是 BizHawk 常态。
--
-- 【相位约定】`emu.frameadvance()` 返回后截图 = 该 emu 帧渲染完成的画面（单回调两相位
-- 的收口侧）；与 rust_screenshot `capture <N>` 产物同相位（findings §10.1 实测校验）。

local dir = os.getenv('BH_SHOT_DIR')
local frames_env = os.getenv('BH_SHOT_FRAMES')
if dir == nil or frames_env == nil or frames_env == '' then
  console.log('ERROR: 需要 BH_SHOT_DIR 与 BH_SHOT_FRAMES')
  client.exit()
  return
end

local wanted = {}
local last = 0
for tok in string.gmatch(frames_env, '[^,]+') do
  local n = tonumber(tok)
  if n ~= nil and n >= 0 then
    wanted[n] = true
    if n > last then last = n end
  end
end
local end_env = tonumber(os.getenv('BH_SHOT_END') or '')
if end_env ~= nil and end_env > last then last = end_env end

local made = 0
local frame = 0
while frame <= last do
  emu.frameadvance()
  frame = frame + 1
  if wanted[frame] then
    client.screenshot(string.format('%s/biz_f%05d.png', dir, frame))
    made = made + 1
  end
end
console.log(string.format('shots done: %d / target %d, last frame %d', made, #wanted, last))
client.exit()
