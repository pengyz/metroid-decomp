#!/usr/bin/env python3
"""可玩端到端门冒烟的判定器（#106）。

输入：game-shell-pc `--state-out` 产出的逐帧 CSV + 换房后那一帧的 PPM 截图。
输出：一行结论 + 退出码（0 PASS / 3 未触发门 / 4 未换房 / 5 黑屏）。

抽成独立文件的原因：判定规则要能被 `tools/playable_door_smoke.sh --self-test`
用合成样本逐条验（正样本 + 三条失败分支），否则规则坏了没人发现。
"""

import csv
import sys

SENTINEL = {255, 241, 243}   # 转场哨兵房间号，不算「新房间」
WINDOW = 90                  # 门事件后允许多少帧完成换房
MIN_NONBLACK = 0.05          # 主体区非黑像素占比下限（正常帧实测 0.155–0.169）
TOP, BOTTOM = 32, 232        # 跳过顶部 HUD 带，只看画面主体
W = 256


def num(row, key):
    try:
        return int(str(row.get(key, '0')).strip(), 0)
    except ValueError:
        return 0


def judge(csv_path, shot):
    rows = list(csv.DictReader(open(csv_path, newline='')))
    if not rows:
        print('[FAIL:EMPTY_CSV] 逐帧状态为空')
        return 1

    trigger = next((i for i, r in enumerate(rows) if num(r, 'door_entry') != 0), None)
    door_data_max = max(num(r, 'door_data') for r in rows)

    if trigger is None:
        print(f"[FAIL:NO_DOOR_TRIGGER] {len(rows)} 帧内 door_entry($56) 恒 0、"
              f"door_data($58) 峰值={door_data_max} ⇒ Samus 从未踩到门格")
        return 3

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
        return 4

    frame, (room, mx, my) = switch
    print(f"[换房] f={frame} 新房间={room} MapPos={mx}/{my}")

    if not shot:
        print('[FAIL:NO_SHOT] 没找到换房后的截图，无法判「新房间可渲染」')
        return 5

    data = open(shot, 'rb').read()
    body = data.split(b'\n', 3)[3]
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
        return 5

    print('[PASS] 走进门 → 触发门 → 换房 → 新房间可渲染')
    return 0


def main(argv):
    if len(argv) < 2:
        print('用法：playable_door_judge.py <state.csv> [shot.ppm]', file=sys.stderr)
        return 2
    return judge(argv[1], argv[2] if len(argv) > 2 else '')


if __name__ == '__main__':
    sys.exit(main(sys.argv))
