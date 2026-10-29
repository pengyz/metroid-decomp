# R9 对抗复核 verdict——0a99466f（end_of_room 恢复 stream_column_kick——接缝根因修）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：pass**

## 1. 前提重验核 —— PASS

- 移出 kick 的旧前提（kick 列写 bog 2 帧与 more_init frame-gate [1,0,0,1] 冲突）
  已被 d667d6cc 证伪（列写 bog 归因错误，stream_column_kick 现价=0 bog）——
  前提失效论证成立。
- **实测复验**：`cargo test -p game-core more_init` →
  `more_init_declares_frame_gates` **绿**（mainloop.rs:1745
  `assert_eq!(m.iter_gates, vec![1,0,0,1])`）——该测经 more_init→SetupRoom→
  draw_room→end_of_room（kick 现已内含）全径，[1,0,0,1] 无扰=载房窗 bog 序列
  不变的直接实证。

## 2. asm 核 —— PASS

- EndOfRoom（$EAF4，prg7_engine.asm:8727-8745）：双写后 `and #$02/bne Lx224` →
  **jmp EndOfRoomHorizontal（$E70C，:7913）**。
- EndOfRoomHorizontal（:7913-7928）实录：`ldx ScrollDir/cpx TempScrollDir/bne RTS`
  防重 + `(ScrollX&0xF8)/8` 列 idx + `jmp UpdateNameTable`——**首列流送 kick 确为
  载房窗内件**（$EAF4 尾径直落）。rust `stream_column_kick`（physics.rs:1133-1153）
  同构：reset_addr_latch+列 idx 同式+30 行纵列写（src=RoomRAM/dst=PPU 名表，
  EB85/E562/E560 式）。end_of_room 调用点防重条件恒真（$4A 刚写=ScrollDir）——
  与 asm 载房窗径等价。
- findings §6 对读一致（行号锚+全径描述相符）。

## 3. L4 声明核 —— PASS（fresh 复跑逐值复现）

- **l380 = 626**（1105→626，-43%）、l580 = **498**（977→498）、l780 = **531**
  （1010→531）、l950 = **490**（969→490）——全线腰斩复现；f950 = **732**
  （1134→732）。逐值与声明一致（diff 图左缘柱补齐与 kick 机制相符：迟到首列
  随房即出）。
- 矩阵 **match 34 / diff 19（34/53）无回退**；门链五点（002 f420-790）match、
  落地三点（001 f715/725/735）match；f1000/f1150 未动（时序类记账面）。
- 右滚 10 点 0px 保持（接缝探针面）。

## 4. 必跑验证 —— PASS

- `cargo test --workspace`：**441 passed / 0 failed**（game-core **407**——任务书
  口径 ✓）。
- `python3 gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 前提重验（冲突前提失效+more_init [1,0,0,1] 无扰实测） | pass |
| 2 asm 核（$EAF4→$E70C kick=载房窗内件；rust 同构） | pass |
| 3 L4（l380=626 等逐值复现；34/53 无回退；门链/落地保持） | pass |
| 4 验证（441/0（game-core 407）；gates 13/0） | pass |

**verdict = pass**。

Evidence: prg7_engine.asm :8727-8745/:7913-7928（EndOfRoom→Horizontal kick 全径）；
physics.rs :1133-1153（stream_column_kick 同构）；room.rs end_of_room 恢复 diff
（0a99466f）；mainloop.rs :1737-1745（more_init [1,0,0,1] 测绿）；shots_rust/matrix.md
（2026-10-29 fresh run-all 逐值）；cargo test（441/0）；gates check（PASS=13 FAIL=0）。
