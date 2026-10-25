# ObjAnim 偏移真值对账工单（r26 轮 12 分析定谳）

> 状态：**真值已定/链调谐待对账**。asm 为主（refs/m1disasm）。

## 1. 真值（A 级——common_sprite_data 逐标签）

| 标签 | ROM 址 | ObjectAnimIndexTbl($8572) 偏移 |
|---|---|---|
| ObjAnim_SamusRun | $8572 | **0x00** |
| ObjAnim_SamusJump | $857E | **0x0C** |
| ObjAnim_SamusSalto | $8580 | **0x0E** |
| ObjAnim_SamusJumpPntUp | $85A7 | **0x35** |
| ObjAnim_SamusFront | $8576 | 0x04（r25 已用 ✓） |

## 2. 现链调谐态（问题）

- handler.rs 的 OBJ_ANIM_SAMUS_RUN/JUMP 常量=旧估（0x12/0x14）——**与真值不符**
- 动画比较/写面按旧常量调谐（测试期望 0x0C/0x35=真值 ✓ 但链输出≠）
- CD16 三件端口（轮 12 试）触发中立跳 CFA9 爆点（真常量下）——旧常量下路径不同

## 3. 对账步骤（下轮）

1. 常量换真值（Run=0/Jump=$0C/Salto=$0E/JumpPntUp=$35）
2. 动画比较/写面全件核（samus_run/samus_stand/transition 族的 anim 常量引用）
3. 测试期望对账（0x0C/0x35=真值 ✓ 应自洽）
4. CD16 三件端口（CF88/D09C/CF2E）+CFA9 中立跳收件
5. 对拍收零（samus-move3）

## 4. 相关件（已读 asm）

- SetSamusRunAccel（$CCB7）：SamusAccelX←RunAccelerationTbl[x]（右=$30/左=$D0——$CCC0）
- Table06（$D096）：3B=ObjAnim_SamusJump 偏移（转身径）
- SamusRun_CheckHorzMovementMidair（$CF88）/SamusJump_CheckFire（$D09C）/LCF2E（$CF2E）
