# 玩家死亡 / 复位链（B4-b）

> 状态：v0.1（2026-11-25，task #54）。证据基线 `refs/m1disasm/SRC/prg7_engine.asm`（社区匹配反汇编，D3 投影）。
> 范围：Samus 从「血量被打到 0」到「进入 game over 例程」的**计时与状态迁移**，以及它和 `title/` 复位面的接口边界。
> 不在范围：音效内容本身（B4-a / #53）、`PrepareGameOver` 与标题画面的实现（title 面）、敌/子弹侧伤害来源判定。

## 1. 链图（A 级：asm 行号逐段可查）

```
[伤害径] SubtractHealth ; 07:CE92
   ├─ HealthChange == 0 ──► 直接返回（不扣血）
   ├─ IsSamusDead ; 07:CE84 == 1 ──► ClearHealthChange（死了不再扣）
   ├─ EndTimer ≠ 0 或 MotherBrainStatus ≠ 0 ──► 走特殊径
   ├─ Varia(gear&$20) ──► HealthChange 半伤
   ├─ Base10Subtract（BCD 逐位）──► 判死两条并集：
   │      ① Health&$F0 | (Health+1) == 0      ② 借位（borrow == 1）
   └─ 死 ──► Health ← 0 / Health+1 ← 0
              status ← sa_Dead(7)
              SFXTriInitFlags |= $80（SFX_SamusDie）
              jmp SetSamusExplode ; 07:CC8B
                     jumpDisplcmnt ← $50 / animResetIndex ← $32 / animIndex ← $32
                     animDelay ← 0 / ObjectCounter ← 0
[爆炸期] DrawObject 特例 ; 07:DE62（对象槽 0 + ObjPlace7）
   ObjectCounter++ 每帧；低 2 位索引 ExplodeRotationTbl（00 80 C0 40）改镜像位
   ObjectCounter == $19(25) ──► status ← sa_Dead2(8) / animDelay ← $28(40 帧) / ClearObjectCntrl
[死二段] SamusDead2 ; 07:D41F ──► dec Samus.animDelay（每帧一次）
[引擎门] GameEngine 尾 LC95F ; 07:C95F
   status == sa_Dead2 且 animDelay == 0 ──► SilenceMusic
        MotherBrainStatus == $0A ──► inc MainRoutine（结局径）
        否则 ──► SetTimer($04, routine 4) ＝ 40 帧后跑 PrepareGameOver
[重开] PrepareGameOver（title 面）
```

## 2. 计时表（帧数＝NTSC 60Hz）

| 段 | 帧数 | 依据 |
|---|---|---|
| 爆炸动画（`sa_Dead`） | 25 帧（`ObjectCounter` 0→$19） | `;$DE62` `cmp #$19` |
| 死二段静默（`sa_Dead2`） | 40 帧（`animDelay=$28`） | `;$DE7A` `lda #$28` + `D41F dec` |
| game over 延迟 | 40 帧（`SetTimer($04, 4)`，A×10） | `;07:C97B` |
| 死亡后 **不再**触发伤害 | 由 `IsSamusDead` 门覆盖全伤害径 | `;07:CEA3`（SubtractHealth 内） |

**i 帧与击退的关系（容易误解）**：`CheckHealthStatus` 在 CE1E 先写 `speedY=$FD`/`AccelY=$38`（击退初速），
**之后**才 `jsr IsSamusDead`；死时只跳过「`SamusInvincibleDelay` 递减 + 击退 X 速衰减」，直跳 `CheckHealthBeep`。
所以「死了还写击退初速」是保真行为，不是缺陷。

## 3. backlog 序号 23 / 24 的落点

| 序号 | ROM 例程 | 本轮 | 落点与说明 |
|---|---|---|---|
| 23 | `IsSamusDead ; 07:CE84` | **做** | `crates/game-core/src/samus/death.rs::is_samus_dead`。语义＝`status ∈ {7, 8, $FF}`；`$FF`（`sa_Begin`）也算死是 ROM 的 `cmp #sa_Begin` 收尾造成的 quirk，保真保留。接线点：`samus/physics.rs::check_health_status` 的 CE1E 死门（此前是 `DIVERGENCES.push(0x0CE25)` 占位臂，已消除）。其余调用者（`SubtractHealth` 已在 `weapon/projectile.rs` 内联；`CollisionDetection` 属 `world/` 写面）不在本卡范围。 |
| 24 | `StoreObjectPositionToTemp ; 07:E8BE` | **做（typed 形态）** | `samus/physics.rs::store_object_position_to_temp` + `ObjectPositionTemp`。`ObjectCheckMoveVertical ; 07:E7B5` 首件即调它，故其入口改为一次快照取值、后续读快照，而不是三次分散读 Samus 字段。其它调用者（`SamusRoll`、`PlaceBulletAtArmCannon`、`UpdateBullet_Move`、`ObjectCheckMoveLeft/Right` 的横径）在 weapon/ 或未端口面，本卡不改。 |

## 4. 已知差异 / 未覆盖（显式声明）

1. **零页临时槽不物化**：ROM 把位置三字节写进 `Temp0B_PositionHi($0B)`/`Temp08_PositionY($08)`/`Temp09_PositionX($09)`；Rust 侧以 `ObjectPositionTemp` 值传递代替。写点与所有读点之间没有别的件写这三个槽，故行为等价；**零页字节面不出现这三个中间值**（若某天做 L2 全字节对比，这里是已知差异，不是缺陷）。
2. **SFX 只置旗**：`SFXTriInitFlags |= $80` 是采样面；音效引擎发声在 B4-a（#53）面，本卡只保证旗位与 asm 一致。
3. **`PrepareGameOver` 与标题画面**：属 title 面，本卡只保证引擎门在 `animDelay==0` 时发出 `SetTimer($04, 4)`，不验证标题画面内容。
4. **`MotherBrainStatus == $0A` 的结局径**：Rust 侧在 `frame/tick.rs` 引擎尾件已存在（本条不新增证据），Tourian 结局的整体行为不在本卡验收。

## 5. 与 `title/` 的接口边界

- 死亡链**只写** `Samus.{status,animDelay,ObjectCounter}`、`Health*`、`SFXTriInitFlags`，以及引擎门的 `Timer3/NextRoutine/MainRoutine`。
- 从「引擎门」到「标题画面复位」的交接点是 `MainRoutine ← 9`（`PrepareGameOver`）；`title/` 面接手后如何清 page/滚动/密码系统状态不在本卡。
- 因此本卡不持有「重开一定回到标题且状态干净」这个断言；可验证的部分是 §2 的帧数与 §1 的迁移顺序。

## 6. 复现（不需要 emulator）

```bash
CARGO_TARGET_DIR=/tmp/v54t cargo test -q -p game-core --lib samus::death
# 5 tests: 真值表 / 死门 / 活体对照 / HP→0 触发链 / Dead2 计时
python3 gates/cli.py check      # 见 task #54 交付说明的 gate 口径
```

## 7. 证据指针

- `refs/m1disasm/SRC/prg7_engine.asm`：`IsSamusDead`(07:CE84)、`CheckHealthStatus`(07:CE1E-CE2A)、`CheckHealthBeep`(07:CE66)、`SubtractHealth`(07:CE92)、`SetSamusExplode`(07:CC8B)、`SamusDead2`(07:D41F)、引擎门(07:C95F)、爆炸绘制(07:DE62)、`StoreObjectPositionToTemp`(07:E8BE)、`ObjectCheckMoveVertical`(07:E7B5)。
- Rust 落点：`crates/game-core/src/samus/death.rs`、`samus/physics.rs`（死门 + 位置快照）。
- 冒烟：`cargo test -q -p game-core --lib samus::death` → `5 passed`。
