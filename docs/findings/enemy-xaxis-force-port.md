# 敌 x 轴力端口（EnemyForceSpeedTowardsSamus）取证与暂缓收口

状态：**实现完成但未入库**（复现补丁见 `enemy-xaxis-force-port.patch.b64`），因撞 L4 ratchet 一格回退而暂缓。
日期：2026-11-14。

## 1. 为什么做这一件

A 重锚（输入窗后移出 440 帧开场演出，commit `28335bc2`）把 22 个此前"空对空"的采集点变成真分歧。逐帧归因后，`case-jumpledge-probe` 六格与 `case-scrollseam-probe` 若干格的残余像素全部落在**敌精灵**上（Samus 七列逐帧已与金腿全等）。

继续下钻到第一分歧帧 `f928`：`En` 槽 2（`$0420`）出生那一帧，rust 的 `data05 = 0x01`、金腿 `0x00`。

asm 定谳（`07:EB06 GetEnemyData` → `LEB6E` → `07:FB7B InitEnemyFacingDirectionAxisAndDelay`）：

- 出生径的写点只有 `$040F`（specialAttribs）、`$0400`（status）、`$0401`、`$0404`、EnsExtra 的 `$6AF4`/`$6AFB`，**没有 `$0408`（movementIndex）**；
- `asl $0405,x`（ROM `EB7E = 1E 05 04`）配 `ror $0405,x`（`FB7B = 20 B0 80 / 7E 05 04`），净效果 `data05' = (L977B[type].bit7 << 7) | (data05 & $7F)`——**bit0 保持旧值**。

推论：`data05` 的 bit0（朝向）**不是出生时定的**，由运行期 `EnemyForceSpeedTowardsSamus` 的 x 轴力段写。旧 rust 代码用 `EnsExtra.status & ScrollDir & 1` 塞 bit0，是在 x 轴力缺失时的**补偿**——横滚房里恰好与"朝左"同形，所以一直是假合。

## 2. asm 逐段（A 级）

`EnemyForceSpeedTowardsSamus ; 07:F6DF`–`07:F70F`：

- `EnsureObjectNotHurt`：`specialAttribs` bit7 置 ⇒ 跳过 x 轴力（冻住/受击态不动）；
- x 轴力：`and #~$01 / sta data05`（清朝向位）→ `jsr SetEnemyFacingTowardsSamus`（写 bit0）→ `lsr A`（朝向进 carry）→ `lda speedX / ror A` → `eor speedX` → `bpl exit`（朝向与速度同向 ⇒ 不动）→ `lda L968B[type] / and #$20` 置 ⇒ exit → `jsr LoadTableAt977B`（carry = 表值 bit7）置 ⇒ 先补子像素再翻速（`00:81DA flipSpeed`）。

`SetEnemyFacingTowardsSamus`：横滚房且异名表 ⇒ 朝相邻名表方向；同名表/竖滚房 ⇒ 比 `En.x` 与 `Samus.x`，相等时 `inc Enemy82`（滑帧用），返回 `En.x >= Samus.x`。

## 3. 实现（补丁里的两个函数）

- `crates/game-core/src/frame/mainloop.rs::enemy_force_speed_x_axis(mem, x, attribs)`：按上面逐段；
- `crates/game-core/src/frame/mainloop.rs::enemy_facing_towards_samus_bit(mem, x)`：朝向判定；
- 同时按 §1 勘误两处出生面：`data05` 只改 bit7、删 `+0x08` movementIndex 播种。

## 4. 实测（单案 float 口径）

| 变体 | jumpledge | scrollseam | case-002 |
|---|---|---|---|
| A：x 轴力 + 出生勘误 | 5/8（1215/1240/1260 **转 match**，1190 9087→274） | 6/14（**1240 由 match 变 184px**） | 2/8（1110 248） |
| B：只上 x 轴力，保留旧补偿 | 不到 match（163/208/312） | 与 A 逐格相同（1240 仍 184） | 同 A |

结论：**+3 收益来自出生勘误，1240 的回退来自 x 轴力端口**；两者必须一起上，故整批暂缓。

全矩阵（双腿 fresh）：`case 15 · 分母 132 · match 110 · diff 22 · blocked 0`（基线 108/24）——净 +2，但门禁原文：

```
L4 逐采集点 ratchet 回退 1 处（match 集合只增不减 + blocked 只减不增，非聚合率）:
case-scrollseam-probe/1240:scroll-l40: match 集合缩水（基线 match → 现态 diff）——回退 FAIL
```

## 5. 那 1 格的归因进度（未闭合）

- 幽灵是 3 条 OAM：`y=0x7B/0x83/0x8B, tile=0xCE/0xCF/0xEF, x=0x26`，rust 腿独有，其后的条目与金腿逐条相同但整体后移 3。
- 来源槽 `$0450` 在rust 腿绘制时刻：`status=0x02`（Active）、`data05=0x1A`、`hi=0x00`、`y=0x01`、`x=0x00`；同一帧末两侧该槽 `status` 均为 0。
- `nt = (PPUCTRL ^ ScrollDir) & 1 = 1`，而 `hi = 0` ⇒ `DeleteOffscreenRoomSprites` 的判据 `(nt ^ hi) & 1 == 0` 不成立 ⇒ 该槽不被删；金腿侧则在该帧之前已删。
- 尚未定谳的分支：①该槽是"陈旧/垃圾槽"（`y=0x01 x=0x00 hi=0x00` 形似未初始化），rust 腿某处把它置成 Active；②`hi` 与 `nt` 的一帧相位差（与已知 #8 `$00FF` bit2 同族）。
- 已排除：`draw_enemy` 只有一个调用点（`update_enemy_common_no_move_no_anim`，守卫 `$6AF4 + x != 0`）；`check_update_name_table` 确实每像素调 `setup_room`（`crates/game-core/src/samus/physics.rs:1566`）。

## 6. 收口顺序（下次接手照此做）

1. 先定谳 `$0450` 槽的来源：在 `load_enemy` 与 `delete_offscreen_room_sprites` 两处加一次性 trace，记录该槽每帧 `status/hi/y/x` 与写入者；
2. 若属垃圾槽 ⇒ 修初始化（对比金腿 `EraseAllSprites`/`DestroyEnemies` 的覆盖范围）；
3. 若属 `hi` 相位 ⇒ 与 #8 的 `$00FF` bit2 一并修（同源，一处修两处受益）；
4. 修好后重跑 `case-scrollseam-probe` 确认 1240 回 match，再跑全矩阵、`gates/cli.py check`，最后把补丁整体入库（commit message 引 `07:F6DF`、`07:FB7B`、`07:EB7E` 三锚 + `Evidence`/`Confidence` 裸行）。

复现：`base64 -d docs/findings/enemy-xaxis-force-port.patch.b64 > /tmp/x.patch && git apply /tmp/x.patch`（补丁以 base64 存放——原始 diff 会有大量重复行，触发 commit-msg 的「文档重复行」闸）。

## 7. 教训

- **补偿巧合第四次**：缺失的机制（x 轴力）被另一处写点（出生 bit0）以"看起来等效"的方式掩盖，只有在补齐真机制、并且输入窗后移让敌真正动起来之后才暴露。
- 所以"修复导致某格回退"要先问：**旧的 match 是真等价，还是两个错互相抵消**。这一问决定了该修机制还是该回退修复。

## 8. 2026-11-14 追加：幽灵来源定谳（不是陈旧槽）

上节两条候选分支（陈旧槽 / hi 与 nt 相位差）**都不成立**。加码取证后结论是另一件更大的缺口：

- 在 `draw_enemy` 装入帧号后逐帧打印 f1235–f1245：锈腿只画一格，`slot=0x20`，`st=0x02 y=0x88 x=0x90 hi=0x01 anim=0x1B..1D`；
- 同帧窗口基线（补丁回退后）**一次都没画**（无任何输出）⇒ 是三格 OAM 的来源；
- 金腿时间线（临时 watch 用例，`name case-seamdbg4`，已删）：`f927 st=1 y=0x38 x=0x8C hi=1` → `f1066 st=2 d05=0x1B` → **`f1188 st=0 y=0xC4 x=0xA2 d05=0x16`（爆体清槽）**；
- 锈腿同槽在 f1240 仍 `st=2`，且位置从 f1235 到 f1245 **恒为 y=0x88 x=0x90**（一格没动）。

定谳：这是 `EnExtra.type=4` 的 **Skree**，金腿的移动指令解释器把它从 (0x38,0x8C) 一路推到 (0xC4,0xA2) 并在 f1188 爆体；锈腿的 Skree **从出生起就没执行过一条移动指令**（位置恒定），因此既不会走到爆体分支（`instr >= 0x11` 那条 `SkreeBlowUpIntoProjectiles` 永远进不去），也不会像金腿那样在 f1188 消失。

补丁只是让它的朝向位变对（`data05` bit0 由 x 轴力写），于是它在锈腿里从"不动也不画"变成"不动但画出来"——0px 变 184px 的真正原因不是相位，而是**敌人移动指令引擎缺件**。

据此调整收口顺序：

1. 先补 Skree（乃至通用）**移动指令执行**：`movementIndex` → 指令流 → `ApplySpeedToPosition` → 逐像素移动 → `movementInstrIndex` 推进；
2. 爆体分支已有（`crates/game-core/src/frame/mainloop.rs` 的 `SkreeBlowUpIntoProjectiles`），引擎补上后应自然到达；
3. 再上 x 轴力补丁，`case-scrollseam-probe/1240` 应随 Skree 准时消失而回 match；
4. 这条修好还会连带改善 `case-jumpledge-probe` 的 148px（同一只 Skree 的四个弹）与 `case-002` 的万像素面。

也就是说：**x 轴力批次本身没有错，它只是把一个更早的缺件照出来了**。补丁继续留在本目录，等移动引擎落地后再一起入库。
