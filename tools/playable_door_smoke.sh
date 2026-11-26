#!/usr/bin/env bash
# 可玩端到端冒烟：开局 → 走进门 → 换房（#106）。
#
# 与 tools/door_entry_repro.sh（#102，poke 注入版）的区别：本脚本**不许用 --poke**，
# 全程只用键盘脚本，验的是「玩家自己能不能走到门格并换房」这条完整链路。
#
# 用法（在 wt-roll 或主线检出里）：
#   tools/playable_door_smoke.sh              # 跑阶段 1（横滚门，Brinstar map5 右门）
#   tools/playable_door_smoke.sh --stage 2    # 跑阶段 2（另一方向门，需 #104 给可达门）
#   tools/playable_door_smoke.sh --self-test  # 用合成样本验判定器本身（不跑游戏）
#   DOOR_SHOTS=/tmp/smoke tools/playable_door_smoke.sh
#
# 退出码即结论（0 才算通过）：
#   0  触发 → 换房 → 新房间可渲染
#   3  NO_DOOR_TRIGGER：$56 全程 0（Samus 从未踩到门格 ⇒ 可进入性/门瓦问题）
#   4  NO_SWITCH：门事件已触发，但窗口内 RoomNumber/MapPos 未变（滚屏/SetupRoom 断链）
#   5  BLACK_SCREEN：换房后首帧主体区非黑像素 < 5%（黑屏/未画房）
#   6  STAGE_UNAVAILABLE：该阶段的门尚未确定（显式红，不静默跳过）
#   1  运行失败（二进制/参数/读不到产物）
#
# 现状（lane/roll HEAD 5b85aeb8，即 #102 已修、#104 未落地）：
#   阶段 1 预期 `NO_DOOR_TRIGGER`（map5 右门在中高台，跳峰 26px、门台高差约 63px，
#   纯键盘到不了门格）。#104 落地后本脚本应转 0；转绿之前它一直是红灯，这是设计意图。
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
JUDGE="$HERE/playable_door_judge.py"
STAGE=1
SELFTEST=0
SCRIPT_OVERRIDE=""
FRAMES_OVERRIDE=""
SHOT_FRAMES_OVERRIDE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --stage) STAGE="${2:-1}"; shift 2 ;;
    --script) SCRIPT_OVERRIDE="${2:-}"; shift 2 ;;
    --frames) FRAMES_OVERRIDE="${2:-}"; shift 2 ;;
    --shot-frames) SHOT_FRAMES_OVERRIDE="${2:-}"; shift 2 ;;
    --self-test) SELFTEST=1; shift ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done

# ── 判定器自检：合成样本覆盖 1 正 + 3 失败分支 ──────────────────────────────
# 只验「判定规则对不对」，不碰游戏。合成 CSV 的列与 game-shell-pc 的 --state-out 一致。
if [ "$SELFTEST" = 1 ]; then
  T="$(mktemp -d)"
  trap 'rm -rf "$T"' EXIT
  hdr='frame,joy,game_mode,main_routine,room,map_x,map_y,scroll_x,scroll_y,door_entry,door_delay,samus_xy,samus_status,load_phase,door_data,door_scroll_status'

  # 正样本：f1 无门、f2 触发门、f3 房间换到 24（MapPos 6/14）
  {
    echo "$hdr"
    echo '1,0,0,2,255,5,14,0,0,0,0,080/0B0,0,32,0,0'
    echo '2,0,0,2,255,5,14,0,0,129,18,0DC/0C1,32,32,2,0'
    echo '3,0,0,2,24,6,14,0,0,1,0,0DC/0C1,0,32,2,1'
  } > "$T/pos.csv"
  # 负样本：门从未触发
  {
    echo "$hdr"
    echo '1,0,0,2,255,5,14,0,0,0,0,080/0B0,0,32,0,0'
    echo '2,0,0,2,255,5,14,0,0,0,0,0DC/0C1,32,32,0,0'
  } > "$T/neg.csv"
  # 截断样本：触发了门，但 90 帧内房间没换
  {
    echo "$hdr"
    echo '1,0,0,2,255,5,14,0,0,129,18,0DC/0C1,32,32,2,0'
    echo '2,0,0,2,255,5,14,0,0,1,0,0DC/0C1,0,32,2,1'
  } > "$T/cut.csv"

  # 两个 PPM：正常画面（主体铺 128 灰）与全黑
  printf 'P6\n256 240\n255\n' > "$T/lit.ppm"
  head -c $((256 * 240 * 3)) /dev/zero | tr '\0' '\200' >> "$T/lit.ppm"
  printf 'P6\n256 240\n255\n' > "$T/black.ppm"
  head -c $((256 * 240 * 3)) /dev/zero >> "$T/black.ppm"

  fail=0
  check() {  # check 名称 期望码 csv [ppm]
    local name="$1" want="$2" csv="$3" ppm="${4:-}"
    python3 "$JUDGE" "$T/$csv" ${ppm:+"$T/$ppm"} > "$T/out.txt" 2>&1
    local got=$?
    local line
    line="$(grep -E '^\[' "$T/out.txt" | tail -1)"
    if [ "$got" = "$want" ]; then
      printf '  [ok]   %-22s rc=%s %s\n' "$name" "$got" "$line"
    else
      printf '  [FAIL] %-22s 期望 rc=%s 实得 rc=%s >>> %s\n' "$name" "$want" "$got" "$(tail -1 "$T/out.txt")"
      fail=1
    fi
  }

  echo "[自检] 判定器四态（合成样本，不跑游戏）"
  check PASS 0 pos.csv lit.ppm
  check NO_DOOR_TRIGGER 3 neg.csv
  check NO_SWITCH 4 cut.csv
  check BLACK_SCREEN 5 pos.csv black.ppm
  if [ "$fail" = 0 ]; then
    echo "[SELFTEST:PASS] 四态判定与退出码均符合预期"
    exit 0
  fi
  echo "[SELFTEST:FAIL] 判定器与预期不符（见上）"
  exit 1
fi

case "$STAGE" in
  1)
    # Brinstar：开局做 START，按住 RIGHT 走到底（路线上唯一的门对象在 map5 右缘）。
    FRAMES=2600
    INPUT=$(cat <<'EOF'
9-9 START
18-18 START
200-2600 RIGHT
EOF
)
    SHOT_FRAMES="2400,2500"
    DOOR_NAME="Brinstar map5 右门（横滚门）"
    ;;
  2)
    echo "[STAGE_UNAVAILABLE] 阶段 2（另一方向门）尚未选到目标房间："
    echo "  开局步行可达的房间（map3/4/5 右行、map3/2 左行）里只有 map5 一道门对象，"
    echo "  竖滚门/另一方向门的可达路径要等 #104 的到达性结论。这里显式红，不静默跳过。"
    exit 6
    ;;
  *) echo "阶段只能是 1 或 2（收到 $STAGE）" >&2; exit 2 ;;
esac

# 覆盖项：等 #104 给出真实可达路径的脚本后，可直接复用判定器而不改本文件。
if [ -n "$SCRIPT_OVERRIDE" ]; then
  if [ ! -r "$SCRIPT_OVERRIDE" ]; then
    echo "[FAIL] --script 指定的文件不存在或不可读：$SCRIPT_OVERRIDE" >&2
    exit 2
  fi
  INPUT="$(cat "$SCRIPT_OVERRIDE")"
  DOOR_NAME="${DOOR_NAME}（脚本来自 $(basename "$SCRIPT_OVERRIDE")）"
fi
[ -n "$FRAMES_OVERRIDE" ] && FRAMES="$FRAMES_OVERRIDE"
[ -n "$SHOT_FRAMES_OVERRIDE" ] && SHOT_FRAMES="$SHOT_FRAMES_OVERRIDE"

OUT_DIR="$(mktemp -d)"
SCRIPT="$OUT_DIR/input.txt"
CSV="$OUT_DIR/state.csv"
SHOTS="${DOOR_SHOTS:-$OUT_DIR/shots}"
printf '%s\n' "$INPUT" > "$SCRIPT"

cd "$HERE/.." || exit 2
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}" \
  cargo run -q -p game-shell-pc --bin game-shell-pc -- \
    --headless --script "$SCRIPT" --frames "$FRAMES" \
    --state-out "$CSV" --shot-frames "$SHOT_FRAMES" --shot-dir "$SHOTS" \
    >/dev/null 2>"$OUT_DIR/err.log"
rc=$?
if [ $rc -ne 0 ]; then
  echo "[FAIL] 运行失败（rc=$rc）："
  tail -5 "$OUT_DIR/err.log"
  exit 1
fi
if [ ! -s "$CSV" ]; then
  echo "[FAIL] 没有产出逐帧状态 CSV：$CSV"
  exit 1
fi

echo "[冒烟] 阶段 $STAGE：$DOOR_NAME"
echo "[产物] csv=$CSV shots=$SHOTS"

last_shot="$(ls "$SHOTS"/*.ppm 2>/dev/null | tail -1)"
python3 "$JUDGE" "$CSV" "${last_shot:-}"
