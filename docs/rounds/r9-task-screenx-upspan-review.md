# R9 对抗复核 verdict——dfecadee（敌屏位 $26→SCROLL_X 勘误）+c331c5af（上行臂全距扫）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：pass**

## 1. asm 核 —— PASS

- **敌 sprite 屏位换算**：IsObjectVisible（$DFDF，prg7_engine.asm:6495-6508）实录：
  `lda Temp0B_PositionX / sec / sbc ScrollX / sta Temp0E_ScreenX`——行内注释即
  「Subtract x scroll to find sprite's x position on screen」——**敌/物屏位 x=
  房位−ScrollX($FD)**（constants_ram.asm:796 `ScrollX db ;$FD`）。rust 修
  `read_ram(SCROLL_X)` 同源 ✓。
- **$26 误引考古**：constants_ram.asm:488 `SpareMem26 dw ;$26`——**m1disasm 全源
  零代码引用**（grep 仅 constants 定义行）；引入点 f554727f（sprite 管线端口）
  `scroll_x=read_ram(0x0026)` 无 A 级证据（sprite.rs 勘误注早已证 $25/$26=
  MMC1CTRL_ZP 同族误读）——伪语义引入，勘误正当。
- **上行臂全式**：c331c5af object_check_move_up 补 `rx==0 跳过门+vertical_span_cols
  步 8 走环`——与下行臂（2e7e5435）/asm $E7BD 全式同族同构 ✓。

## 2. 爬行 TDD —— PASS

- `crawl_moves_over_50_frames`：emu f820 实测形敌态（y=45/x=200/data0A=1/
  status=Active）+50 帧 FRAME_COUNT 推进 crawler_ai——**路径长≥5px 锚**
  （振荡爬行净≈0：转向回转形）——AI 执行实证（恒定形 travel=0 即红）。
- x 200/45→199/46 序合理性：下爬步进+沿转向（6/8 步门+四例行）——振荡形与
  travel 断言一致。

## 3. 测有效 —— PASS

- `up_edge_overlap_detected`：check_y=0x50 行、心列（0x41>>3 心）空/侧顶砖 ⇒
  碰撞——回退单点形（只检心列）⇒ 红 ✓。
- `enemy_screen_x_subtracts_scroll_x`：ScrollX 50→60 ⇒ OAM 屏位 −10；旧形减
  $26（与 ScrollX 无关）⇒ 差 0 ⇒ 红——钉屏根因锚 ✓。

## 4. L4 复跑 + FC 相位摆论证 —— PASS（本复核直接实证摆动）

- 本复核 fresh run-all：**48/74（hit-2/dead 回归 ✅match 0px）** vs lead 复跑
  46/74（该两点翻 diff）——**同树两跑点级翻转=FC 相位 run 间摆的直接实证**
  （非产品回退；与 BUG-089 时序类同族）。
- 门链五点/落地三点/右滚 10 点/kill 6 点全保持；jumpledge land 0px 保持。
- `cargo test --workspace`：**452 passed / 0 failed**（game-core **418**=415+3 ✓）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**。

## 裁定

| 点 | 结论 |
|---|---|
| 1 asm 核（$DFDF sbc ScrollX；$26 零引用伪语义；上行臂同族全式） | pass |
| 2 爬行 TDD（50 帧 travel≥5 振荡锚） | pass |
| 3 测有效（up_span 侧顶砖+屏位滚屏锚 mutation） | pass |
| 4 L4（48/74 vs 46/74 点级翻转=相位摆实证；全保持面 ✓） | pass |
| 5 验证（452/0；gates 13/0） | pass |

**verdict = pass**。

Evidence: prg7_engine.asm :6495-6508（IsObjectVisible sbc ScrollX）/:7162-7168；
constants_ram.asm :488/:796；git log -S（f554727f 引入无据）；physics.rs
up_span/crawl 测实读；sprite.rs 勘误注（既有）；shots_rust/matrix.md（本复核
fresh 48/74 vs lead 46/74 点级翻转对照）；cargo test（452/0）；gates check
（PASS=13 FAIL=0）。
