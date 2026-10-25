# 敌显示链工单（r26 轮 5 分析定谳——UpdateEnemyCommon→DrawEnemy）

> 状态：**分析定谳（结构已明/数据面缺口待核）**。asm 为主（refs/m1disasm）。
> 前置：玩法域 goal（第一个房间四流程）——敌显示=打敌人/移动对拍的显示面。

## 1. 显示链结构（asm 证据）

```
UpdateWorld($CB29) → UpdateAllEnemies($F345)（槽环 $50→0×16B）
  → UpdateEnemy($F351)：状态 dispatch（0=Exit/1=Resting/2=Active/3=Explode/4=Frozen/
    5=Pickup/6=Hurt/≥7=RemoveEnemy）
  → Resting 件序（F3BE）：ForceSpeed(F6B9)→Threshold(F75B)→RestingAnim(F682)→
    UpdateEnData1F(F676)→TryBecomingActive(F7BA)【已全端口 ✓】
  → AI dispatch（区 bank $95E5）：type 0-0F→逐型 AI（Zoomer=type5→crawler.asm）
  → UpdateEnemyCommon($F410)：Anim+Move/NoMoveNoAnim/NoMove 三入口
    → tough 调色（specialAttribs bit7→ObjectCntrl=$80|3|PRIORITY）
    → **DrawEnemy($DD8B)**（status≠0 时）
    → isHit/weaponAction 清
```

## 2. DrawEnemy（$DD8B）= sprite 管线会师径

- `animFrame == _id_FrameNone` ⇒ ClearObjectCntrl（空帧径）
- 否则（$DDB0）：位置（Ens.y/x/hi→Temp0A/0B/06）→
  **帧指针 = (AreaPointers_RAM.EnFramePtrTable1($41)/2($43))，`asl/tay` carry 选表！**
  → GetSpriteCntrlData（$DCC3——sprite 域已端口！）
  → **EnPlacePtrTable($45)** 指位件 → 爆径（EnPlace1 时：chunkyExplosionTimer/
    ExplodeRotationTbl/0x19 判→EnemyCompleteDeath）
  → WriteSpriteRAM 族（sprite 管线会师 ✓ 已端口！）

## 3. 数据面缺口（下轮工单）

- **AreaPointers_RAM（$3B-$48=7 字——constants_ram:206 AreaPointersStruct）**
  - $41/$43=EnFramePtrTable1/2（carry 选表）；$45=EnPlacePtrTable
  - **旧 copy_area_pointers 的 $8043 源址存疑**（areas_common:90 = CommonJump_SubtractHealth
    ——非拷贝件！）——真源待核（AreaInit/区 bank 指针块真址）
  - 现对拍不受影响（房间走 ROOM_PTR_TABLE 烘焙径 ✓）；**敌显示帧/位表走此面——必须先正源**
- 敌帧/位数据本体（EnFramePtrTable→帧流→EnPlacePtrTable→位流）：烘培面待建（或与
  SPRITE_DATA（$8572 共用精灵数据）会师——指针值核后定）

## 4. 下一步（顺序）

1. AreaPointers 真源核（AreaInit 的指针块拷贝——prg1/prg7 源）→ 烘焙面
2. EnFramePtrTable/EnPlacePtrTable 数据体烘焙（或与 SPRITE_DATA 会师裁定）
3. DrawEnemy 端口（sprite 管线复用+敌帧/位表源）→ 敌显示可见
4. Zoomer CrawlerAI 主体（6/8 门+四朝向+JumpByRTSToMovementRoutine 移动件）
5. → 移动对拍（samus-move3）可达 → 打敌人/吃能力/过门场景对拍
