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
INVALID_ROOM=0
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
    --check-invalid-room) INVALID_ROOM=1; shift ;;
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
  # 方向不符：同一份「右横滚」样本按竖滚档（3,4）判，必须报 WRONG_DIRECTION 而不是 PASS
  python3 "$JUDGE" --expect-codes=3,4 "$T/pos.csv" "$T/lit.ppm" > "$T/out.txt" 2>&1
  got=$?
  if [ "$got" = 6 ] && grep -q 'WRONG_DIRECTION' "$T/out.txt"; then
    printf '  [ok]   %-22s rc=%s %s\n' WRONG_DIRECTION "$got" "$(tail -1 "$T/out.txt")"
  else
    printf '  [FAIL] %-22s 期望 rc=6 实得 rc=%s >>> %s\n' WRONG_DIRECTION "$got" "$(tail -1 "$T/out.txt")"
    fail=1
  fi
  if [ "$fail" = 0 ]; then
    echo "[SELFTEST:PASS] 四态判定与退出码均符合预期"
    exit 0
  fi
  echo "[SELFTEST:FAIL] 判定器与预期不符（见上）"
  exit 1
fi

START_ARGS=""
# ── 负对照：非法房号必须被拒绝（rc=7），不许静默当 0xFF 空房号 ──────────────
if [ "$INVALID_ROOM" = 1 ]; then
  T="$(mktemp -d)"
  trap 'rm -rf "$T"' EXIT
  printf '9-9 START\n' > "$T/in.txt"
  cd "$HERE/.." || exit 2
  CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}" \
    cargo run -q -p game-shell-pc --bin game-shell-pc -- \
      --headless --script "$T/in.txt" --frames 200 \
      --start-room 0xF5 --start-frame 60 --state-out "$T/s.csv" \
      >"$T/out.log" 2>"$T/err.log"
  rc=$?
  if [ "$rc" != 0 ] && grep -q '房间号不被接受' "$T/err.log"; then
    echo "[CHECK:PASS] --start-room 0xF5 被拒绝（rc=$rc，stderr 有「房间号不被接受」）"
    exit 0
  fi
  echo "[CHECK:FAIL] 期望 rc≠0 且报「房间号不被接受」，实得 rc=$rc"
  tail -3 "$T/err.log"
  exit 1
fi

case "$STAGE" in
  1)
    # Brinstar 房 5（MapPos 5/14）右门：纯键盘版。
    # 起点不在门台高度、且该房左右两端都是普通滚屏格，所以用 --start-pos 只把 Samus
    # 摆到门台（y=0x6F）上，其余全部靠按键：B 开锁门 → RIGHT 走进去。
    # 实测（A 级）：f940 门槽 0/0/3/0（Open）、f948 $56=129($81) $58=0x11、
    # f967 RoomNumber 255→24、MapPos 5/14→6/14、f968 起 ScrollX=2,4,6…
    FRAMES=1700
    INPUT=$(cat <<'EOF'
9-9 START
18-18 START
200-1700 RIGHT
925-945 B
EOF
)
    START_ARGS="--start-pos 0xD0,0x6F --start-frame 900"
    SHOT_FRAMES="1200"
    DOOR_NAME='Brinstar 房 5 右门（横滚门，\$56=\$81）'
    EXPECT_CODES="1,2"
    ;;
  2)
    # 同一对房间的回程门：房 5 右门进去到 Room24（MapPos 6/14），再从这个门走回来。
    # 回程触发的方向码是 $84（=4|$80，竖向上滚），与阶段 1 的 $81（右横滚）不同向。
    # 实测（A 级）：f2490 $56=132($84) $58=0x11、f2509 MapPos 6/14→5/14 房间 24→19。
    FRAMES=2900
    INPUT=$(cat <<'EOF'
9-9 START
18-18 START
200-1700 RIGHT
925-945 B
1200-2400 LEFT
2450-2470 B
2480-2900 LEFT
EOF
)
    START_ARGS="--start-pos 0xD0,0x6F --start-frame 900"
    SHOT_FRAMES="2850"
    DOOR_NAME='Room24 左门回程（竖向滚屏门，\$56=\$84）'
    EXPECT_CODES="3,4"
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
    --headless --script "$SCRIPT" --frames "$FRAMES" $START_ARGS \
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
python3 "$JUDGE" --expect-codes="$EXPECT_CODES" "$CSV" "${last_shot:-}"
