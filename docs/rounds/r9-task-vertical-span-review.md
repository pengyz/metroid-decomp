# R9 对抗复核 verdict——2e7e5435（垂向检 x 全距走环——跳台阶嵌砖根因修）

- 复核员：verifier-scroll（证伪立场，非实施方）
- 日期：2026-10-29
- **verdict：pass**

## 1. asm 核 —— PASS

- **radiusX=0 跳过门**：CheckMoveVertical（$E7BD，prg7_engine.asm:8042-8045）
  `lda Objects.0.radiusX,x（fallthrough）/ bne Lx198 / sec / rts`——0⇒放行。
  rust `if rx == 0 { return true }` 同形。
- **全距走环**：:8048-8055 左界=PositionX−radiusX+左余块 inx；LE7E6 环（:8085-8108）
  NextPointXOffset=#$08 逐块 +8 扫（:8080 `ldy #$08`）。
- **GetNumBlocksToCheck（$E8CE）公式五段逐一对读**：①`(−left_rem)&7`
  （`eor #$FF/adc #1/and #7`）=右余 ②diameter=`tya asl`（=2·radiusX）
  ③`sbc right_rem`+借位 `adc #$08` ④`lsr×3`⇒整块 ⑤右余 inx+左 rem inx 合计
  ——rust `vertical_span_cols` 五段逐条同形；与 `horizontal_span_rows`（physics.rs:995）
  结构全同（x/ry 轴对换）✓。

## 2. 旧单点形考古 —— 漏修非有意（PASS）

- `git log -S`：全距形源自 **8e4710cc**（goal-r16「水平碰撞全径（多行扫）」——
  仅横面）；竖向接线在 **77d6a3d5**（门数据副作用件），其 commit 自述
  「横径行循环（r16 多行扫）+竖向 down/up 全部接线」——竖向以单点形接线，
  **无任何证据表明有意省略全距** ⇒ 漏修定性成立，2e7e5435 补齐=对称性修复。

## 3. 测有效性 —— PASS

- `edge_overlap_detected`：全域 walkable ⇒ 放行（true）；0x40 块砖置于 x+rx 侧块
  （心块仍空）⇒ 碰撞（false）。**mutation**：回退单点形（只检心块）⇒ 第二断言
  红——锚即被修语义。
- 旧测重锚合规：原测补 `SAMUS_RADIUS_X=8` + 证伪更正原地注（radiusX=0 跳过门
  会使旧设 rx=0 隐形通过）——§3.6 合规。

## 4. L4 + 验证 —— PASS

- fresh run-all：**match 42 / diff 25（42/67）无回退**；jumpledge 7 点 0px match
  （game-in/walk-mid/pre-jump/takeoff/rise/peak/**land**——嵌砖修后落地帧即对齐）；
  land-settled（800）=221px 为新点诚实残差（未锚定新点，闸在案提示锁定）。
- 门链五点（002 f420-790）match、落地三点（001 f715/725/735）match、
  f1000/f1150 未动。
- `cargo test --workspace`：**445 passed / 0 failed**（game-core **411**=410+1 新测，
  任务书口径 ✓）。
- `gates/cli.py check`：**PASS=13 FAIL=0，CHECK: PASS**（l4 闸 ratchet 守住在案）。
  - 复核过程记录：首次 check 与我并行跑的 cargo/L4 竞态报 FAIL（同 r9 已登记
    artifact）——干净顺序复跑 PASS，非产品分歧。

## 裁定

| 点 | 结论 |
|---|---|
| 1 asm 核（$E7BD/$E8CE 五段公式+跳过门；rust 同构） | pass |
| 2 考古（8e4710cc 横面/r16；77d6a3d5 竖接线单点=漏修） | pass |
| 3 测有效（edge_overlap mutation+旧测重锚合规） | pass |
| 4 L4（42/67；jumpledge land 0px；444→445/0；gates 13/0） | pass |

**verdict = pass**。

Evidence: prg7_engine.asm :8026-8110（CheckMoveVertical 全径）/ :8430 段
（GetNumBlocksToCheck $E8CE）；physics.rs vertical_span_cols/horizontal_span_rows/
edge_overlap_detected；git log -S（8e4710cc/77d6a3d5）；shots_rust/matrix.md
（fresh 42/67）；cargo test（445/0）；gates check（PASS=13 FAIL=0）。
