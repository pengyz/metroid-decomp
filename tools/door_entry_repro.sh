#!/usr/bin/env bash
# 门入场 → 房间切换 的可复跑验收命令（#102）。
#
# 为什么需要 `--poke`：从通电开局用走路+跳跃可达的范围内没有可站上去的门格
# （唯一找到的 Brinstar map5 右门在中高台，跳跃峰值 26px、平台高差 63px），
# 所以「走进门」这一步无法用纯按键脚本复现。本脚本把 `SamusDoorData($58)=2`
# 在第 1500 帧注入一次，等价于「Samus 已踩在门格瓦上」（$58 的写入面在
# `collision_point_samus`，与本脚本要验的门过渡链无关），然后看门链自己走不走。
#
# 用法（在 wt-roll 或主线检出里）：
#   tools/door_entry_repro.sh            # 默认 2400 帧
#   tools/door_entry_repro.sh 3000       # 指定帧数
#   DOOR_SHOTS=/tmp/door-shots tools/door_entry_repro.sh   # 截图落指定目录（默认临时目录）
#
# 判据（三条缺一不可）：
#   ① 注入后 `$56`（DoorEntryStatus）= `方向码|$80`，即 0x81/0x82/0x83/0x84；
#   ② 注入后 ≤ 40 帧内 RoomNumber 或 MapPosX 发生变化（门滚完、邻房装载）；
#   ③ 新房间那一帧画出来了（f1520 的 PPM 至少 2 种颜色——不是空屏）。
#   落盘截图：注入前 f1499、注入后 f1520，路径在末尾打印。
# 现状（HEAD c240e1b7）：①读到 `0x80`、②不发生 ⇒ 打印 FAIL 并给根因指针。
set -uo pipefail

FRAMES="${1:-2400}"
POKE_FRAME=1500
OUT_DIR="$(mktemp -d)"
SCRIPT="$OUT_DIR/poke.txt"
CSV="$OUT_DIR/state.csv"
SHOTS="${DOOR_SHOTS:-$OUT_DIR/shots}"

cat > "$SCRIPT" <<'EOF'
9-9 START
18-18 START
200-1500 RIGHT
EOF

cd "$(dirname "$0")/.." || exit 2
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}" \
  cargo run -q -p game-shell-pc --bin game-shell-pc -- \
    --headless --script "$SCRIPT" --frames "$FRAMES" \
    --poke "${POKE_FRAME}:0x58=2" --state-out "$CSV" \
    --shot-frames "$((POKE_FRAME - 1)),1520" --shot-dir "$SHOTS" >/dev/null 2>"$OUT_DIR/err.log"
rc=$?
if [ $rc -ne 0 ]; then
  echo "[FAIL] 运行失败（rc=$rc）："
  tail -5 "$OUT_DIR/err.log"
  exit 1
fi

# CSV 列：frame,joy,game_mode,main_routine,room,map_x,map_y,scroll_x,scroll_y,
#         door_entry,door_delay,samus_xy,samus_status,load_phase,door_data,door_scroll_status,...
base=$(awk -F, -v f="$POKE_FRAME" 'NR>1 && $1<=f {r=$5; m=$6"/"$7} END {print r" "m}' "$CSV")
base_room=${base%% *}
base_map=${base#* }

entry=$(awk -F, -v f="$POKE_FRAME" 'NR>1 && $1>=f && $10!=0 {print $10; exit}' "$CSV")
entry=${entry:-0}

# 注入后 40 帧内第一次「房间号或地图格」变化（与注入前最后一帧比）。
changed=$(awk -F, -v f="$POKE_FRAME" -v br="$base_room" -v bm="$base_map" '
  NR>1 && $1>f && $1<=f+40 && ($5 != br || ($6"/"$7) != bm) {print $5" "$6"/"$7; exit}
' "$CSV")

# 判据③：新房间帧是否真的画出来（PPM 里不同颜色数 ≥2）。
shot_ok=0
shot_note="（未生成）"
if [ -f "$SHOTS/f01520.ppm" ]; then
  shot_note=$(python3 - "$SHOTS/f01520.ppm" <<'PYEOF'
import sys
p = sys.argv[1]
b = open(p, 'rb').read()
# P6\n W H\n255\n 头，之后是 RGB 像素
parts = b.split(b'\n', 3)
body = parts[3] if len(parts) > 3 else b''
px = {body[i:i+3] for i in range(0, min(len(body), 3 * 256 * 240), 3)}
print(f"颜色数 {len(px)}")
sys.exit(0 if len(px) >= 2 else 1)
PYEOF
  ) && shot_ok=1 || shot_ok=0
fi

printf '[读数] 注入前：room=%s map=%s；注入后首个门事件 $56=%s\n' "$base_room" "$base_map" "$entry"
if [ -n "$changed" ]; then
  printf '[读数] 注入后 40 帧内：room=%s map=%s\n' "${changed%% *}" "${changed#* }"
else
  printf '[读数] 注入后 40 帧内：房间号/MapPos 均未变化\n'
fi

ok=0
case "$entry" in
  129|130|131|132) ok=$((ok + 1)) ;;
esac
[ -n "$changed" ] && ok=$((ok + 1))
[ "$shot_ok" -eq 1 ] && ok=$((ok + 1))

printf '[截图] 注入前 f%05d：%s；注入后 f01520：%s（%s）\n' \
  "$((POKE_FRAME - 1))" "$SHOTS/f0$((POKE_FRAME - 1)).ppm" "$SHOTS/f01520.ppm" "$shot_note"

if [ "$ok" -eq 3 ]; then
  echo '[PASS] 门入场 → 房间切换成功（$56 带方向码、房间号/MapPos 变化、新房间帧可渲染）'
  exit 0
fi
echo '[FAIL] 门链没有走到切房。若 $56=128（0x80），根因见 docs/findings/door-entry-no-room-switch.md'
echo '       （SamusInDoor $8B74 的 ora #$80 操作数是方向码 X，不是 $56）' 
exit 1
