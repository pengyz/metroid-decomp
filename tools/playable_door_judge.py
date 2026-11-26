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


def door_code(row):
    """`$56` 的方向码：低 7 位（未置 $80 旗时为 0，进门后置旗）。"""
    return num(row, 'door_entry') & 0x7F


def judge(csv_path, shot, expect_codes=None):
    rows = list(csv.DictReader(open(csv_path, newline='')))
    if not rows:
        print('[FAIL:EMPTY_CSV] 逐帧状态为空')
        return 1

    def ok(r):
        if num(r, 'door_entry') == 0:
            return False
        return expect_codes is None or door_code(r) in expect_codes

    trigger = next((i for i, r in enumerate(rows) if ok(r)), None)
    door_data_max = max(num(r, 'door_data') for r in rows)

    if trigger is None:
        if expect_codes is None:
            print(f"[FAIL:NO_DOOR_TRIGGER] {len(rows)} 帧内 door_entry($56) 恒 0、"
                  f"door_data($58) 峰值={door_data_max} ⇒ Samus 从未踩到门格")
            return 3
        else:
            seen = sorted({door_code(r) for r in rows if num(r, 'door_entry') != 0})
            print(f"[FAIL:WRONG_DIRECTION] 期望方向码 {sorted(expect_codes)}，实际只见到 {seen}"
                  f"（$58 峰值={door_data_max}）⇒ 门触发了但方向不是这一档")
        return 6

    trig = rows[trigger]
    before = (num(trig, 'room'), num(trig, 'map_x'), num(trig, 'map_y'))
    # $56 非零序列（去重压缩：只记「值×连续帧数」），供卡面第 1 条核对方向码 1..5 的推进
    seq = []
    for r in rows[trigger:]:
        v = num(r, 'door_entry')
        if v == 0:
            if seq:
                break
            continue
        if seq and seq[-1][0] == v:
            seq[-1][1] += 1
        else:
            seq.append([v, 1])
    seq_txt = ' → '.join(f'{v}×{n}' for v, n in seq[:8]) + (' …' if len(seq) > 8 else '')
    print(f"[触发] f={num(trig, 'frame')} $56={num(trig, 'door_entry')} "
          f"$58={num(trig, 'door_data')} 旧房间={before[0]} MapPos={before[1]}/{before[2]}")
    print(f"[方向码序列] $56: {seq_txt or '（无）'}")

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
    code = door_code(trig)
    if expect_codes is not None and code not in expect_codes:
        print(f"[FAIL:WRONG_DIRECTION] 方向码 {code} 不在期望集合 {sorted(expect_codes)}")
        return 6
    print(f"[换房] f={frame} 新房间={room} MapPos={mx}/{my} 方向码={code}")

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
    args = [a for a in argv[1:] if not a.startswith('--')]
    expect = None
    for a in argv[1:]:
        if a.startswith('--expect-codes='):
            expect = {int(x, 0) for x in a.split('=', 1)[1].split(',') if x.strip()}
    if not args:
        print('用法：playable_door_judge.py [--expect-codes=1,2] <state.csv> [shot.ppm]',
              file=sys.stderr)
        return 2
    return judge(args[0], args[1] if len(args) > 1 else '', expect)


if __name__ == '__main__':
    sys.exit(main(sys.argv))
