# 玩家受伤与击退链（B4 剩余行为面）

> 任务 #123：ROM 取证 + 实现/否证。**结论：主链已端口**；敌撞径另有 2 处 A 级缺件（§5），
> 两处都在 `crates/game-core/src/weapon/projectile.rs`，需写面授权后一行修（本卡已提交
> 给 PMO 裁决，未擅自改）。

## 1. 结论摘要

| 环节 | ROM 入口 | Rust 落点 | 状态 |
|---|---|---|---|
| 敌体撞萨（伤害源 1） | `CollisionDetectionEnemy_ReactToCollisionWithSamus ; 07:F282` | `weapon/projectile.rs:uw_collision_detection`（敌×萨段 `:558-640`） | **已端口，2 缺口**（§5） |
| 敌弹撞萨（伤害源 2） | `CollisionDetectionEnProjectile_ReactToCollisionWithSamus ; 07:F2ED` | 同上（弹×萨段 `:660-706`） | 已端口 |
| 伤害置值 | `SetHealthChangeToAreaEnemyDamage ; 07:F306` | 敌弹径 `:703` 用 `area_header().enemy_damage`；**敌撞径 `:610` 写死 `$0080`** | 敌撞径与 ROM 不符（§5.1） |
| 扣血 | `SubtractHealth ; 07:CE92` | `weapon/projectile.rs:714 subtract_health` | 已端口（Varia/BCD/死亡支） |
| 受击面（i 帧/击退/闪） | `CheckHealthStatus ; 07:CDFA` | `samus/physics.rs:1149 check_health_status` | 已端口 |
| 低血蜂鸣 + 清 `isHit` | `CheckHealthBeep ; 07:CE66` | `samus/physics.rs:1014 check_health_beep` | 已端口（音效面走 `$0683` 旗） |
| 死亡判定谓词 | `IsSamusDead ; 07:CE84` | `samus/death.rs` | 已端口 |

## 2. 调用链

```
每帧 UpdateWorld（07:CB29）
└─ CollisionDetection（07:F034）
   ├─ 环头：SamusKnockbackIsBomb($73)←$FF、SamusHurt010F($010F)←$FF        （:10062-10064）
   ├─ 敌×萨（i 帧门 + IsSamusDead 门，:10071-10096）
   │  └─ CollisionDetectionEnemy_ReactToCollisionWithSamus（07:F282）
   │     ├─ SetEnemyTouchingSamusFlags（F2E8）：敌 isHit |= (DistHi^3)     ← 敌侧标记
   │     ├─ IsScrewAttackActive → 螺旋转有效：敌 weaponAction←wa_ScrewAttack，不伤萨
   │     ├─ 敌 status ≥ Frozen(4) ⇒ RTS（Frozen/Pickup/Hurt 免伤）        （F289）
   │     ├─ SamusHurt010F ← EnsExtra.type                                  （F298）
   │     ├─ L968B[type] & $10 ⇒ RTS（该类敌接触不伤萨）                    （F2A2）
   │     ├─ SetSamusIsHitByEnemy（F338）：Samus.isHit |= DistHi<<3
   │     └─ jmp SetHealthChangeToAreaEnemyDamage（F306）：HealthChange←AreaEnemyDamage
   ├─ 敌弹×萨（i 帧门 + 死门，:10176/:10202）
   │  └─ CollisionDetectionEnProjectile_ReactToCollisionWithSamus（F2ED）
   │     ├─ 弹 isHit |= DistHi
   │     ├─ 螺旋转有效 ⇒ RTS
   │     ├─ SamusHurt010F←$80；Samus.isHit |= (DistHi^3)<<3
   │     └─ jmp SetHealthChangeToAreaEnemyDamage（F306）
   └─ 尾：SubtractHealth（CE92）
└─ SetSamusData（CD6D）→ CheckHealthStatus（CDFA）→ LavaAndMoveCheck（E269）→ …
```

**同帧顺序**：碰撞径先置 `Samus.isHit`/`HealthChange` 并扣血（`SubtractHealth` 在
`uw_collision_detection` 尾部），`CheckHealthStatus` 在之后的 `SetSamusData` 里读
`isHit&$20` 才置 i 帧/击退速度。⇒ Rust 不能把「置 i 帧」写进碰撞径（旧形偏离已在
`projectile.rs:628` 注释登记）。

## 3. 字段表（A 级：`refs/m1disasm/SRC/constants_ram.asm`）

| 地址 | 名 | 语义 | 写者 | 读者 |
|---|---|---|---|---|
| `$006E/$006F` | `HealthChange`（word） | 本帧待扣/回血量 | 伤害源（F306 / 熔岩 / 敌人弹 / 拾取） | `SubtractHealth`（CE92） |
| `$0070` | `SamusInvincibleDelay` | i 帧计数（受击置 `$32`=50） | `CheckHealthStatus`（CDFA）、门入场 | 碰撞径 i 帧门、CDFA |
| `$0072` | `SamusKnockbackDir` | 击退方向：`$FF`=不推，`0`=左，`1`=右 | CDFA | CDFA（X 速符号） |
| `$0073` | `SamusKnockbackIsBomb` | 击退样式：`0`=垂直，`1`=斜向（`KnockSamusBackIfCollided` 依 `Temp0F_DistX`） | `KnockSamusBackIfCollided`（F311） | CDFA（镜像到 `$77`） |
| `$0077` | `SamusKnockbackIsBomb77` | `$73` 的镜像；bit7=已受击（闪烁+清 `isHit` 时基） | CDFA | CDFA（闪烁/`SamusDoor` 判） |
| `$0106/$0107` | `Health`（BCD 双字节） | 低/高字节各两位十进制 | `SubtractHealth`/`AddHealth` | 显示、判死 |
| `$010F` | `SamusHurt010F` | 受击源标记（敌 type / `$E0` 击退径 / `$80` 敌弹） | 碰撞径 | 调试/旧链路 |
| `$0300` | `Samus.status` | `7`=Dead 触发死门 | `SubtractHealth` 死支 | 全局 |
| `$0308/$0309` | `Samus.speedY / speedX`（整） | 击退初速 `$FD` / X 推速 | CDFA | 积分器 |
| `$0314/$0315` | `SamusAccelY / AccelX` | 击退重力 `$38` | CDFA | 积分器 |
| `$030A` | `Samus.isHit` | bit5=被击、bit4=向下推、bit3=向右推 | 碰撞径（`DistHi<<3`） | CDFA（`&$20` 触发；`bit3` 定方向） |

## 4. 关键公式（A 级：`prg7_engine.asm:3095-3155`、`:3195-3260`）

1. **受击面**（CDFA）：`isHit&$20` ⇒ `$70←$32`、`$72←$FF`、`$77←$73`；`$77≠0` ⇒
   `$72←(isHit&$08)>>3`、`speedY←$FD`、`AccelY←$38`；`IsSamusDead` 非零 ⇒ 跳
   `CheckHealthBeep`（死时不减 i 帧、不推 X）。
2. **i 帧倒数 + X 推速衰减**（CDFA `Lx006`）：`$70≠0` ⇒ `dec $70`；`$72+1==0` ⇒ 跳过推速；
   否则 `A = $70(pre-dec)>>4`（`Adiv16`），`A<3 ∧ AccelX==0` ⇒ `speedX←0`；
   `$72==0` ⇒ `A = -A`（推左）；`speedX←A`。⇒ 推速按 `3→2→1→0` 每 16 帧降一档，共 50 帧。
3. **闪烁**（CDFA `Lx009`）：`$77` bit7 置 ∧ `FrameCount&1==0` ⇒ `animDelay←0`、
   `animFrame←$F7`（`ObjDrawFrame` 跳绘）。
4. **扣血**（CE92）：`HealthChange==0` 直接返回；`IsSamusDead` 或 `EndTimer+1≠$FF` 或
   `MotherBrainStatus≥3` 或**死径已置** ⇒ 清 `HealthChange`；Varia(`gr_VARIA` bit5) ⇒
   `lsr HealthChange`（低字节丢进位；高字节右移时若 bit0 置 ⇒ 低字节 `+$4F+1`）；
   BCD 双字节减（高字节借位并入下一次）；`(Health&$F0)|Health+1==0` **或高字节借位**
   ⇒ `Health←0`、`status←sa_Dead(7)`、`SFX_SamusDie`、`SetSamusExplode`；非死径清
   `HealthChange`。死径**不清** `HealthChange`（原版 quirk）。

## 5. 缺口（A 级，均在 `crates/game-core/src/weapon/projectile.rs`，写面外）

### 5.1 敌撞伤害值写死 `$0080`

- ROM：`07:F306` `lda AreaEnemyDamage / sta HealthChange`、`AreaEnemyDamage+1 / sta HealthChange+1`。
- 五区实值：Brinstar `$0080`、Norfair `$0100`、Kraid `$0200`、Tourian `$0300`、Ridley `$0240`
  （`refs/m1disasm/SRC/prg{1..5}_*.asm` `AreaEnemyDamage:`）。
- Rust `projectile.rs:610-611` 固定 `$6E←$80, $6F←$00`：**仅 Brinstar 正确**；其余四区敌撞伤害与 ROM 不符。
- 修法（一行）：`mem.set_health_change(game_data::assets::area_header(crate::frame::cur_area(mem)).enemy_damage);`
  （与敌弹径 `:703` 同形）。

### 5.2 缺 `L968B` bit4 门

- ROM `07:F2A2-07:F2A5`：`lda L968B,y / and #$10 / bne SetSamusIsHitFlagsY@RTS` ⇒ 该项 bit4 置位时接触不伤萨。
- 五区 `L968B` 表全扫（`prg{1..5}_*.asm` `L968B:`，每区 16 项）：**只有 Tourian 的 type 0、1** 置 bit4。
  ⇒ Rust 在 Tourian 会被这两类敌接触伤害（窄面，但属行为差）。
- 修法：敌撞块在 `EnsExtra.type` 取自后，加 `if l968b[type] & 0x10 != 0 { continue; }`（资产侧需确认现有访问器名）。

### 5.3 已核对无差项

- `SamusHurt010F` 敌撞径写敌 type、敌弹径写 `$80`：与 ROM 一致（`F298` / `F2F6` 前段）。
- i 帧门与死门位置：Rust 在敌×萨环头（`:558`）与弹×萨环头（`:658`）各一处，与 ROM 的
  `:10071-10096` / `:10176-10202` 同构（i 帧门先于死门）。
- 敌 status ≥4 免伤门、螺旋转分岔、`DistHi` 三件（`0x04|Δy符号<<1|Δx符号`）均已端口。

## 6. 单测（本卡新增，`crates/game-core/src/samus/tests.rs`）

| 用例 | 断言面 |
|---|---|
| `damage_reduces_health_bcd` | `Health 0x0030 + HealthChange 0x0005 → 0x0025`，`HealthChange` 清零 |
| `knockback_frames_decay_x_speed` | 受击帧写面（`$70=$31`/`$72=$FF`/`speedY=$FD`/`AccelY=$38`）+ X 推速 `3→2→1→0` 的帧数分布（2/16/16/16=50 帧）；`$72=0` 取负、`AccelX≠0` 跳过 |
| `invincible_frames_block_repeat_damage` | i 帧非零 ⇒ 碰撞径零写（`isHit`/`Health`/`HealthChange` 不变）；i 帧归零 ⇒ `isHit` bit5 置、`Health` 下降 |

## 7. 边界

- 本链只覆盖「活着时的受伤面」。死亡后的 `GameOver`/重开/密码屏在 `crates/game-core/src/title/`
  与 `samus/death.rs`：`SubtractHealth` 只负责置 `status=7` + 起爆炸，`title/` 侧按 `status`
  推进死亡流程；测试 `samus/death.rs:92 dead_gate_skips_iframes_but_keeps_knockback_writes`
  钉住「死门跳过 i 帧倒数但保留击退写」这一 ROM 语义。
- 音效：`SFX_SamusHit/SamusDie/Beep` 走 `$0683`（`SFXTriInitFlags`）/`$0680` 旗面，音频后端
  属 #53；本链只写旗位，不做发声。

## 8. 证据指针

- ROM：`refs/m1disasm/SRC/prg7_engine.asm:10060-10215`（CollisionDetection）、`:10501-10640`
  （F282/F2ED/F306/F311/F338）、`:3082-3160`（CDFA）、`:3195-3260`（SubtractHealth）。
- 资产：`refs/m1disasm/SRC/prg{1..5}_*.asm` `AreaEnemyDamage:` / `L968B:`。
- Rust：`crates/game-core/src/weapon/projectile.rs:502 uw_collision_detection`、
  `:714 subtract_health`、`crates/game-core/src/samus/physics.rs:1149 check_health_status`、
  `:1014 check_health_beep`、`crates/game-core/src/samus/health_change.rs`、
  `crates/game-core/src/samus/death.rs`。
- 既有考证：`docs/findings/bcd-mask-audit-subtracthealth.md`（BCD 域穷举）、
  `docs/findings/morphball-trajectory-root.md:551-581`（伤害置径与 AreaEnemyDamage 编码语义
  未定项）。
