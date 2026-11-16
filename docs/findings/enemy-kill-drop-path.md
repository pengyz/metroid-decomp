# 敌「击杀→掉落物」两处 asm 分支缺失（100% 录像 f2152 定谳）

## 现象

100% 录像（Lord Tom，120 540 帧）对拍首个失配 **f2152 = 204px**，两簇：
`x88-98/y24-39`（61 点）与 `x208-223/y144-167`（143 点）。

OAM 影子（`ram.hex` 的 `$0200`）对读显示 **rust 在 slot0–3 多画 4 枚精灵**
（tile `CC CD DC DD`、x=`$58..$5B`），其后所有槽整体后移 4 格；emu 在末尾多 4 枚
`tile 0x75`。**不是采样偏移**：对象槽 `$0300..$03AF` 十二槽两侧逐字节相同，
分歧只在敌槽 1（`$0410`/`$6B04`）：

```
emu  status=05 (Pickup) animFrame=0x81 animIndex=0x27
rust status=03 (Explode) animFrame=0x60 animIndex=0x42
```

`animFrame=0x60` 是 `EnFramePtrTable1` 索引 96 → `bank1:$A126 = 21 00 00 CC CD DC DD FF`
（ctrl `0x21` / radiusY=0 / radiusX=0）——正是 rust 多画的那 4 枚。

## 根因 1：`ExplodeEnemy ; 07:F611` 的「普通束 ⇒ 立即变掉落物」分支缺失

`refs/m1disasm/SRC/prg7_engine.asm:11202-11230`：

```
ExplodeEnemy: lda #enemyStatus_Explode / sta EnsExtra.0.status,x
  bit Temp0A_EnSpecialAttribs              ; bvs ⇒ miniboss 跳过
  lda Ens.0.weaponAction,x
  cmp #wa_RegularBeam+1 / bcs @endIf_A     ; 非普通束 ⇒ 走爆炸动画
      jsr EnemyCompleteDeath@SkipChunkyExplosion   ; 普通束 ⇒ 立刻变掉落物
      ldx PageIndex
  @endIf_A: jsr GetEnemyTypeTimes2PlusFacingDirection
            lda EnemyDeathAnimIndex,y / jsr InitEnAnimIndex
            sta Ens.0.chunkyExplosionTimer,x
```

`wa_RegularBeam = $01`（`constants.asm:117`）。rust 旧形无此分支 ⇒ 一律播 0x40 帧
爆炸动画 ⇒ 掉落物晚约 64 帧出现。

**修**（`crates/game-core/src/enemy/framework.rs::explode_enemy`）：写 status=3 之后，

```rust
let sa_x = mem.read_ram(0x0400 + px + 0x0F); // specialAttribs
let wa_x = mem.read_ram(0x0400 + px + 0x0E); // weaponAction
if sa_x & 0x40 == 0 && wa_x < 0x02 { enemy_complete_death(mem, x); }
```

**不 return**——asm 随后仍执行 `InitEnAnimIndex` 与 `sta chunkyExplosionTimer`。

## 根因 2：`EnemyCompleteDeath ; 07:DD0D` 的 `type` 覆盖缺失

`07:DD1A-07:DD23`：`jsr ReadTableAt968B / sta Temp00` →
`jsr LoadTableAt977B / and #$20 / sta EnsExtra.0.type,x`，
即 **type 被覆盖成 0 或 `$20`**（`LoadTableAt977B` 返回 `val<<1`，`and #$20` 测的是原字节 bit4）。

紧随其后的 `GetEnemyTypeTimes2PlusFacingDirection` 用的是**覆盖后的 type**。
旧 rust 不写 ⇒ 用真 type=5（Skree）⇒ 死亡动画索引取 0x41 而非 emu 的 0x27
（`EnemyDeathAnimIndex` 32 B = type×2+朝向）。

**修**：在 `enemy_complete_death` 的 `PickupTypeTable` 之前写 `type ← enemy_facing(area)[type] & 0x20`。

## 结果

- f2152：**204 → 99px**；`EnsExtra` 槽 1 与 emu 逐字节相同。
- 剩余 99px 已定界：emu 在 `EnExplosions` 池（`$04C0`，4 槽 × 8 B，额外面 `$6BB4`）里
  放了一个 4 枚精灵的爆炸特效并绘制（`UpdateAllEnemyExplosions ; 07:FA9D` →
  `UpdateEnemyExplosion ; 07:FAB4`，尾 `lda #$80 / sta ObjectCntrl / lda #$03 / jmp AnimDrawEnemy`）。
  **rust 侧整支缺失**（无 spawn、无 update、无绘制）⇒ 下一件补这一对。

## 测试口径更正（两条旧测把缺陷当期望）

`crates/game-core/src/frame/tests.rs::tests_kill_chain` 两条以 `weaponAction=1`（普通束）
断言 `status==3` ——按 asm 应为 **5**（立刻变掉落物）。已按 asm 更正，并给
`collision_ring_links_bullet_to_kill` 钉 `RandomNumber1≥$10`（`EnemyCompleteDeath` 的
1/16「不掉落」支会 `RemoveEnemy` 把 status 清 0，那是正确行为）。

Evidence: asm `07:F611-F623`（prg7_engine.asm:11202-11230）、`07:DD0D-07:DD23`（:5956-5968）、
录像 f2152 两侧 `ram.hex` 的 OAM/敌槽逐字节对读。
Confidence: 90

## 追加：`EnExplosions` 池补齐（三件套），f2152 归零

上节残留的 99px 已修。三件套按 asm 逐字实现，**分三步验证**（每步都单独跑窗口）：

| 步骤 | 内容 | 观察 |
|---|---|---|
| 1 | `ExplodeEnemy` 尾件 07:F62E-F673 的 **spawn** | 池字节出现「幽灵槽」：rust 有、emu 无 |
| 2 | `UpdateAllEnemyExplosions ; 07:FA9D` + `UpdateEnemyExplosion ; 07:FAB4` | 幽灵槽在年龄到期后被 `RemoveEnemy` 收掉 ⇒ 池字节对齐 |
| 3 | 尾件 `lda #$80 / sta ObjectCntrl / lda #$03 / jmp AnimDrawEnemy` | f2152 **99 → 0px** |

第 1 步的「幽灵槽」是重要教训：**只 spawn 不 update 会让槽永久滞留**，于是第 3 步一旦接上绘制，
旧槽会被画出来，反而把本来全绿的 f2144–f2150 打成 12499px（我第一次就是直接三件一起上，被打回）。
三步分开、每步看池字节，才定位到「滞留」而不是「绘制错了」。

实现锚（`crates/game-core/src/enemy/framework.rs`）：
- 池 `$04C0` 4 槽 × 8 B（`y+0/x+1/data05+5/delay+6/quantity+7`），额外面 `$6BB4`
  （`status+0/radiusY+1/radiusX+2/animFrame+3/animDelay+4/resetAnimIndex+5/animIndex+6/hi+7`）。
- `@explosionMovementTable ; 07:FAEA`：`(0,0) / ($0C,$1C) / ($10,$F0) / ($F0,$08)`，按 `quantity*2` 索引。
- 绘制前 `ObjectCntrl←$80`；`AnimDrawEnemy ; 07:F97E` = `UpdateEnemyAnim` + `DrawEnemy`；
  `DrawEnemy ; 07:DD8B` 先比 `animFrame==$FF`（空帧 ⇒ `ClearObjectCntrl`）。

结果：`case-movie-100p-en1` 窗口 f2144–f2152 **5/5 全 0px**；权威矩阵
**15 案/132 格 · match 126 · diff 6 · blocked 0（逐格无回退）**；测试 622→**626**。

下一步首分歧（同一 bisect2 窗口实测）：f2160 8px → f2170/f2190/f2210/f2230 **44px 与 0 交替**
（2 帧动画里有一帧不对），f2240 起稳定 ~240px（常驻对象）。已定界，未修。

Evidence: asm bank07:F611-F673（ExplodeEnemy 尾件）、bank07:FA9D-FAEA（爆炸池 update/绘制/位移表）、
bank07:DD8B（DrawEnemy 空帧门）；录像池字节 `$04C0-$04DF` 与 OAM 逐帧对读。
Confidence: 92
