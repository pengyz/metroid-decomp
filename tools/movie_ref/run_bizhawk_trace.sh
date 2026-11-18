#!/usr/bin/env bash
# tools/movie_ref/run_bizhawk_trace.sh —— BizHawk 2.10(mono) Linux 无头逐帧 RAM 取证配方
#
# 用途：一条命令跑通「加载 .bk2 录像 → 逐帧导 RAM 面 → 落 jsonl」，把本机踩过的三个坑
#       （OpenAL 卡死主线程 / 相对路径被 cd 偷换 / 退出不干净）固化在脚本里，避免重踩。
#       坑的机理与证据见 docs/knowledge/gotcha-bizhawk-headless-mono-setup.md
#       与 docs/findings/movie-bizhawk-6725m.md §2/§3。
#
# 用法：
#   tools/movie_ref/run_bizhawk_trace.sh <movie.bk2> [frames] [out.jsonl]
#     movie.bk2   录像绝对/相对路径（必填）
#     frames      导出帧数，默认 1200（产出 frames 行，f=0..frames-1）
#     out.jsonl   输出路径，默认 analysis/traces/bizhawk-6725M.jsonl
#
# 可用环境变量覆盖：
#   BIZHAWK_DIR=/tmp/bh           BizHawk 解包目录（含 EmuHawkMono.sh）
#   ROM=rom/Metroid\ \(USA\).nes  ROM 路径
#   LUA=tools/movie_ref/bizhawk_ref.lua
#   TIMEOUT=240                   外层超时秒数（全片 28408 帧实测约 8 min ⇒ 传 2000+）
#
# 判成功的标准：输出行数 == frames。**不要看退出码**：
#   BizHawk 在 shutdown routines 末尾会自己 SIGKILL 本进程（日志原话
#   "BizHawk has completed its shutdown routines, killing process..."）⇒ 退出码 137，
#   20 帧实测 wall 1.9s 就已 137，与 timeout 无关。属预期，不是失败。
#   timeout 只是保险（全片段落跑长了用来兜底）。
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIZHAWK_DIR="${BIZHAWK_DIR:-/tmp/bh}"
LUA="${LUA:-$REPO_ROOT/tools/movie_ref/bizhawk_ref.lua}"
ROM="${ROM:-$REPO_ROOT/rom/Metroid (USA).nes}"
TIMEOUT="${TIMEOUT:-240}"

MOVIE="${1:-}"
FRAMES="${2:-1200}"
OUT="${3:-$REPO_ROOT/analysis/traces/bizhawk-6725M.jsonl}"

die() { echo "ERROR: $*" >&2; exit 2; }

[ -n "$MOVIE" ] || die "用法: $0 <movie.bk2> [frames] [out.jsonl]"
[ -f "$MOVIE" ] || die "录像不存在: $MOVIE"
[ -f "$ROM" ] || die "ROM 不存在: $ROM（BYO-ROM：仓库不含版权数据）"
[ -f "$LUA" ] || die "Lua 脚本不存在: $LUA"
[ -x "$BIZHAWK_DIR/EmuHawkMono.sh" ] || die "找不到 $BIZHAWK_DIR/EmuHawkMono.sh（用 BIZHAWK_DIR= 指定解包目录）"
command -v xvfb-run >/dev/null || die "缺 xvfb-run（apt-get install xvfb）"
command -v mono >/dev/null || die "缺 mono（BizHawk 的 EmuHawkMono.sh 依赖 net472 运行时）"

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"

# 绝对路径化：EmuHawkMono.sh 会 cd 到 $BIZHAWK_DIR，相对路径会落错地方（与 Mesen2 CWD 偷换同族坑）。
abspath() { (cd "$(dirname "$1")" && printf '%s/%s' "$(pwd)" "$(basename "$1")"); }
MOVIE_ABS="$(abspath "$MOVIE")"
ROM_ABS="$(abspath "$ROM")"
LUA_ABS="$(abspath "$LUA")"
OUT_ABS="$(abspath "$OUT")"

echo "== BizHawk 取证 =="
echo "  core     : 由 $BIZHAWK_DIR/config.ini 的 PreferredCores 决定（NES=quickerNES，见 findings §2）"
echo "  movie    : $MOVIE_ABS"
echo "  rom      : $ROM_ABS"
echo "  frames   : $FRAMES  → 期望 $FRAMES 行（f=0..$((FRAMES - 1))）"
echo "  out      : $OUT_ABS"
echo "  timeout  : ${TIMEOUT}s"

# 关键环境：ALSOFT_DRIVERS=null 掐掉 OpenAL（否则卡死主线程、Lua 一行不执行）；
#           SDL_AUDIODRIVER=dummy 掐掉 SDL 音频；xvfb-run 提供无头 X。
(
  cd "$BIZHAWK_DIR" || exit 2
  ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy BH_FRAMES="$FRAMES" BH_OUT="$OUT_ABS" \
    timeout "$TIMEOUT" xvfb-run -a ./EmuHawkMono.sh \
      --chromeless --lua "$LUA_ABS" --movie "$MOVIE_ABS" "$ROM_ABS"
)
rc=$?

rows=0
[ -f "$OUT_ABS" ] && rows=$(wc -l <"$OUT_ABS")
echo "== 结果 =="
echo "  退出码   : $rc  （137 = BizHawk shutdown 后自杀，预期；124 = timeout 兜底生效）"
echo "  输出行数 : $rows  （期望 $FRAMES）"

if [ "$rows" -eq "$FRAMES" ]; then
  echo "  STATUS: OK（产物完整；退出码不参与判定——137 是 BizHawk 自杀）"
  exit 0
elif [ "$rows" -gt 0 ]; then
  echo "  STATUS: PARTIAL（有产物但不足帧数：可能 timeout 太短，或用例本身提前结束）" >&2
  exit 1
else
  echo "  STATUS: FAIL（无产物：先查 OpenAL 是否被掐掉 —— 症状是进程不退且 Lua 一行不执行）" >&2
  exit 1
fi
