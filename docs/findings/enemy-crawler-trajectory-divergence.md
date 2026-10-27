# 敌 Crawler 轨迹首分歧 f799 考证（goal-r19）

## 状态：假设期（未定谳）——本文全部为「实测数据+待验证假设」，无结论

## 实测（emu watch 逐帧，case-001，2026-10-27）

敌 slot4（右柱 Zoomer，type=5）轨迹 emu vs rust：

| 帧 | emu (y,x,orient) | rust (y,x,orient) |
|---|---|---|
| f798 | (40,188,or0) | (40,188,or0) |
| **f799** | **(40,189,or0)** | (40,188,or0) ← **首分歧：emu 动 1px，rust 未动** |
| f819 | (44,200,or1) 拐角下行✓ | (41,190,or3) 误入上向 |

级联：rust 1px 落后 → 提前撞柱角 → InsideCorner(碰撞)orient 0→3（上）——而 emu 是
OutsideCorner 路径 0→1（下）正确拐弯。f950/f1200/f820 全部由此轨迹差主导。

## CrawlerAI($9A4C) 逐指令反汇编（ROM 直读——该段 m1disasm 未标注，grep 零命中）

```
$9A4C jsr $8009   ; ShouldCrawlerMove(F852)
$9A4F and #$03
$9A51 beq $9A87   ; gate==0 ⇒ 跳到 anim+common（跳过移动）✓ rust 门语义对齐
$9A53 lda $81     ; EnemyStatusPreAI
$9A55 cmp #$01 / beq $9B44   ; Resting 早退
$9A59 cmp #$03 / beq $9B49
$9A5D lda $6AF4,x / cmp #$03 / beq $9A87
$9A64 lda $040A,x / and #3 / cmp #1 / bne $9A7E   ; orient==1?
$9A6D ldy $0400,x / cpy #$E4 / bne $9A7E          ; y==$E4?（熔岩线）
$9A74 jsr $9ABD / lda #3 / sta $040A,x / bne $9A84 ; lava turnaround
$9A7E jsr $9AE2   ; MOVE（经 CrawlerMovementRoutinesTable $8048）
$9A81 jsr $9AA8   ; InsideCornerCheck（move 失败才动作：碰撞⇒orient−1；离屏⇒flip）
$9A84 jsr $9AC6   ; OutsideCornerCheck（恒跑：探 (orient+1) 成功⇒orient+1，探移提交）
$9A87 lda #3 / jsr $800C(UpdateEnemyAnim) / jmp $8006(common)
```

## 待验证假设（下轮主攻）

**H-A（主嫌）**：emu 在 gate=0 帧（采样 FC=$10）x 动了 1px ⇒ 存在未建模移动源。
候选：① 采样 FC 与逻辑帧 FC 差一（watch 采样在 spin-exit，FC 递减时点未核）
② OutsideCorner 探移在 gate≠0 帧的提交语义（探移 1px 即数据面移动）
③ ShouldCrawlerMove 的 X 输入（PageIndex=0x40？或 PageIndex 此帧≠slot4）

**H-B**：1px 漂移独立于 gate——rust enemy_move_one_pixel_* 与 asm $83F5 族在某边界差
1px（如 radius/块界判）。

**验证法**：emu 全帧 x 增量时刻表 + 对应帧逻辑 FC + gate 值三列表格化（现 trace 已有
frameCount/en4x，analysis/traces/case-001-*.frames.jsonl），找出 x++ 帧的 gate 分布——
若 gate≠0 帧也 x++ ⇒ H-A②；若全在 gate==0 ⇒ H-A①采样时差。

## 已确认对齐面（勿重复排查）
- gate==0 跳过移动语义：rust `crawler_ai` `if gate==0 {common;return}` ✓ 与 $9A51 对齐
- UpdateEnemyAnim(E094) 走表 ✓（r31 已端口，cadence=3）
- CrawlerReorientSprite($9A8F+$9AA0 表) ✓（r31 已端口）
- InsideCorner/OutsideCorner 结构 ✓（$9AA8/$9AC6 端口，探移提交语义已核）
- 敌渲染管线（DrawEnemy/EnFramePtrTable/enemy_gfx 烘焙）✓（帧 $58 tiles CC/CD/DC/DD 实测）


## 数据质量边界（2026-10-27 补）

f431-446 段 $0441 是**每帧 -1 的递减计数**（29→14）——敌未生成/未更新期的脏值（intro 期
GameEngine 未跑），**不是位置**。可信窗口：敌实际移动段（f795-830 实测 x=188→200 稳定
单调）。下轮重分析须先定位敌 spawn/首更新帧（$0441 值稳定的起点），只取该窗口做
x++/gate 分布表。

## 关联
- 前置：docs/findings/anim-state-machine-model.md（decomp eefc9c3d）
- 下游：f820(359)/f950(1270)/f1200(15929) 全由本轨迹差主导
