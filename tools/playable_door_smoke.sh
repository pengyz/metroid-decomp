#!/usr/bin/env bash
# 可玩端到端冒烟：开局 → 走进门 → 换房（#106）。
#
# 与 tools/door_entry_repro.sh（#102，poke 注入版）的区别：本脚本**不许用 --poke**，
# 全程只用键盘脚本，验的是「玩家自己能不能走到门格并换房」这条完整链路。
#
# 用法（在 wt-roll 或主线检出里）：
#   tools/playable_door_smoke.sh              # 跑阶段 1（横滚门，Brinstar map5 右门）
#   tools/playable_door_smoke.sh --stage 2    # 跑阶段 2（另一方向门，需 #104 给可达门）
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

STAGE=1
while [ $# -gt 0 ]; do
  case "$1" in
    --stage) STAGE="${2:-1}"; shift 2 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done

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

OUT_DIR="$(mktemp -d)"
SCRIPT="$OUT_DIR/input.txt"
CSV="$OUT_DIR/state.csv"
SHOTS="${DOOR_SHOTS:-$OUT_DIR/shots}"
printf '%s\n' "$INPUT" > "$SCRIPT"

cd "$(dirname "$0")/.." || exit 2
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

# 判定：全部逻辑放在一处（python），bash 只负责跑与透传退出码。
last_shot="$(ls "$SHOTS"/*.ppm 2>/dev/null | tail -1)"
python3 - "$CSV" "${last_shot:-}" <<'PY'
import csv, sys

SENTINEL = {255, 241, 243}      # 转场哨兵房间号，不算「新房间」
WINDOW = 90                     # 门事件后允许多少帧完成换房
MIN_NONBLACK = 0.05             # 主体区非黑像素占比下限（正常帧实测 0.155–0.169）
TOP, BOTTOM = 32, 232           # 跳过顶部 HUD 带，只看画面主体

csv_path, shot = sys.argv[1], sys.argv[2]

def num(row, key):
    try:
        return int(str(row.get(key, '0')).strip(), 0)
    except ValueError:
        return 0

rows = list(csv.DictReader(open(csv_path, newline='')))
if not rows:
    print('[FAIL:EMPTY_CSV] 逐帧状态为空')
    sys.exit(1)

trigger = next((i for i, r in enumerate(rows) if num(r, 'door_entry') != 0), None)
door_data_max = max(num(r, 'door_data') for r in rows)

if trigger is None:
    print(f"[FAIL:NO_DOOR_TRIGGER] {len(rows)} 帧内 door_entry($56) 恒 0、"
          f"door_data($58) 峰值={door_data_max} ⇒ Samus 从未踩到门格")
    sys.exit(3)

trig = rows[trigger]
before = (num(trig, 'room'), num(trig, 'map_x'), num(trig, 'map_y'))
print(f"[触发] f={num(trig, 'frame')} $56={num(trig, 'door_entry')} "
      f"旧房间={before[0]} MapPos={before[1]}/{before[2]}")

switch = None
for r in rows[trigger:trigger + WINDOW]:
    cur = (num(r, 'room'), num(r, 'map_x'), num(r, 'map_y'))
    if cur == before:
        continue
    if cur[0] in SENTINEL and cur[1:] == before[1:]:
        continue   # 装载中的哨兵号，不是新房间
    switch = (num(r, 'frame'), cur)
    break

if switch is None:
    print(f"[FAIL:NO_SWITCH] 门事件已触发，但 {WINDOW} 帧内 RoomNumber/MapPos 未变 "
          f"⇒ 滚屏或 SetupRoom 断链")
    sys.exit(4)

frame, (room, mx, my) = switch
print(f"[换房] f={frame} 新房间={room} MapPos={mx}/{my}")

if not shot:
    print('[FAIL:NO_SHOT] 没找到换房后的截图，无法判「新房间可渲染」')
    sys.exit(5)

data = open(shot, 'rb').read()
body = data.split(b'\n', 3)[3]
W = 256
total = nonblack = 0
seen = set()
for y in range(TOP, BOTTOM):
    base = y * W * 3
    for x in range(W):
        i = base + x * 3
        px = (body[i], body[i + 1], body[i + 2])
        total += 1
        if px != (0, 0, 0):
            nonblack += 1
        seen.add(px)
ratio = nonblack / total
print(f"[渲染] {shot} 主体区非黑占比={ratio:.4f} 颜色数={len(seen)}")
if ratio < MIN_NONBLACK:
    print(f"[FAIL:BLACK_SCREEN] 换房后首帧主体区非黑像素 <{MIN_NONBLACK:.0%} ⇒ 黑屏/未画房")
    sys.exit(5)

print('[PASS] 走进门 → 触发门 → 换房 → 新房间可渲染')
sys.exit(0)
PY
