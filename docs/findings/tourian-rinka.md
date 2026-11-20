# Rinka AI 端口（Tourian `03:9A2C`）

**状态**：已端口（`crates/game-core/src/enemy/rinka.rs`，11 条锚测）
**证据级**：A（asm 逐行 + ROM 字节/标签地址），**像素证据：无**（见 §4）

## 1. asm 事实（`refs/m1disasm/SRC/enemies/rinka.asm`，222 行全书）

`RinkaAIRoutine_{AREA}: ; 03:9A2C` 是**按区宏**标签（`{AREA}` 由各区 include 展开），
只被 Tourian bank include（`prg3_tourian.asm` 的敌表第 4 项 = `03:95E5+4*2` 指过来）。

| 段 | asm | 语义 |
|---|---|---|
| 状态门 | `ldy EnsExtra.0.status,x / cpy #enemyStatus_Active / bne @moveRinka` | 非 Active 直接落地移动段 |
| 定速闸 | `dey / cpy EnemyStatusPreAI / bne @moveRinka` | **只在「本帧 Active 且 AI 前快照 `$0081`==Resting」那一帧定速** |
| 清加速 | `lda #$00 / jsr ClearRinkaAcceleration` + `sta subPixelY/subPixelX` | asm 自注「useless，Rinka 不走加速模型」 |
| 位移向量 | `Samus.x - Ens.x → $01`、`Samus.y - Ens.y → $00`，按 `data05` bit0/bit2 取负 | 面向左/上时取负 |
| 归一化 | `$00\|$01` 的 bit7/6/5 逐位右移（`lsr $00 / lsr $01`） | 右移到两者都 < `$20` |
| `SetRinkaSpeed_{AREA}` | `Adiv16_`(lsr×4) → `speed`；`Amul16_`(asl×4) → `speedSubPixel` | 8.4 定点 |
| 二次取负 | `lda #$00 / sbc speedSubPixelX / sbc speedX`（含 Y） | 面向位再取负一次 |
| 移动 | `lda data05 / asl / bmi @endIf_E` | **原字节 bit6 置位 ⇒ 整帧不移动**（30FPS 门） |
| 应用 | `speedSubPixel += EnsExtra.subPixel`；`speed + carry → Temp04/05`；`jsr ApplySpeedToPosition`；失败 `sta status ← 0`；**恒** `jsr LoadEnemyPositionFromTemp_` | 越界语义同 `07:FD8F` |
| 尾件 | `lda #$08 / jmp CommonJump_UpdateEnemyCommon_noMove` | 动画每 8 帧换一次 |

依赖件的 A 级地址：`ClearRinkaAcceleration ; 03:99D1`（`metroid.asm:326-330`，只清
`accelX/accelY`）、`SetRinkaSpeed_{AREA} ; 03:9AF9`、`Adiv16_ ; 03:9B1B`、`Amul16_ ; 03:9B20`、
`LoadEnemyPositionFromTemp__{AREA} ; 02:9A52`（`prg2_norfair.asm:663`：`Temp0B&1 → EnsExtra.hi`、
`Temp08 → y`、`Temp09 → x`）。

## 2. 为什么「只定速一次」是硬约束

定速段写在 `dey / cpy EnemyStatusPreAI / bne @moveRinka` 之后，而 `EnemyStatusPreAI($0081)`
是**本帧 AI 分派之前**写下的状态快照（`enemy/slot_cycle.rs` 在派发前写入）。
因此只有当 Rinka 从 Resting 转成 Active 的那一帧才初始化速度向量。
若按「每帧朝 Samus 飞」实现，Rinka 会变成追踪弹并持续发散（与 `docs/spec/tourian-endgame.md` §13 的
「Rinka 只在 Active 且 prev=Resting 那帧定速一次」一致）。

## 3. 原版 quirk：双重取负 + 逻辑右移 ⇒ 速度不对称（D4 保真，不许"修"）

面向位在**两处**都参与了取负：先在 `$00/$01`（位移）上取负，再在 `speedSubPixel/speed` 上取负。
中间夹着的归一化用的是 `lsr`（逻辑右移），对补码值是**错算**，于是第二次取负并不回到原值：

- `Δx = +$20`、面向**右**：`$01=$20` → 1 次 lsr ⇒ `$10` ⇒ speed=$1、sub=$00 → **1.00 px/帧**
- `Δx = +$20`、面向**左**：`$01=$E0` → 3 次 lsr ⇒ `$1C` ⇒ speed=$1、sub=$C0 → 再取负 ⇒ **−1.25 px/帧**

`tests_rinka::facing_left_keeps_rom_double_negation_quirk` 把这一不对称钉死（断言 `sub == $40`）。

## 4. 诚实边界

- **像素证据：无**。Tourian 房扫被 `entry_preset_unverified_area` 挡住（注入法对非 0 区结构性不可用，
  见 `docs/findings/room-sweep.md` §8.1）；现有一张 20 用例的 L4 矩阵没有一条进 Tourian。
  本卡证据只有 asm 逐行 + 11 条单测锚。
- 未验证项：`data05` bit0/bit2 的运行时取值来源（由谁写朝向位）、Rinka 与 Samus/子弹的碰撞后果
  （走 `enemy/motion.rs` 通用碰撞径，未单独取证）。
- `LoadEnemyPositionFromTemp_` 在**失败**路径也照写回（含 ±`$10` 环绕修正值），本实现按 asm 复刻
  （`apply_speed_to_position_parts` 之后无条件写 `y/x/hi&1`，再按返回值判 `status←0`）。
