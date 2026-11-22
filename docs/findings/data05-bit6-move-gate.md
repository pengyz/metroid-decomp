# `data05` 的 bit6 门：`asl / bmi` 判的是原 bit6（两次同型误读）

**结论（A 级）**：`lda Ens.0.data05,x / asl / bmi <target>` 这个三指令组里，`ASL` 把 **bit7**
移进 C 丢弃，`BMI` 读的 N 来自 **原 bit6**。因此凡以此组为"位移/早退门"的例程，判据都是
`data05 & 0x40`，不是 `& 0x80`。`data05 bit6` 由 `UpdateEnemy_EnData05Bit6 ; 07:F3AA`
每帧写 `((x/16) ^ FrameCount) & 1` ⇒ 该门天然是 30FPS 半速／隔帧执行。

## 1. 两次同型误读（本仓均已修）

| 位置 | 旧形 | asm 锚 | ROM 字节 | 修复 |
|---|---|---|---|---|
| `crates/game-core/src/enemy/pipe_bug.rs:57` | `data05 & 0x80 != 0` | `refs/m1disasm/SRC/enemies/pipe_bug.asm:48-52` | `bank01:9B74 = 0A`（ASL）、`bank01:9B75 = 30 1E`（BMI @exit） | `93bd4ae5` |
| `crates/game-core/src/enemy/metroid.rs:102` | `data05 & 0x80 != 0` | `refs/m1disasm/SRC/enemies/metroid.asm:13-16` | `bank03:97FB = 0A`、`bank03:97FC = 30` | `b59f48fb` |

同族的**正确**写法（已按 bit6，可作为交叉校验）：`crates/game-core/src/enemy/motion.rs:257`
（`EnemyMove ; 01:8058`）、`enemy/framework.rs:587`（`07:F3BE` Resting）、`:611`（`07:F3E6`
Active）、`enemy/rinka.rs:107`、`enemy/kraid.rs:265`。

> **第三方注释陷阱**：`refs/m1disasm/SRC/enemies/pipe_bug.asm:48` 的注释写的是 "bit 7"，
> 与同仓 `rinka.asm:131`、`areas_common.asm:115`、`prg7_engine.asm:10750/10772` 的 "bit 6"
> 相反。判位必须读**指令语义**，不能读注释（AGENTS §3 D5：第三方命名/注释只是线索）。

## 2. 证据：房扫 room 21 的逐帧对读

用例：`room_sweep --rooms 21`（Brinstar，注入 `$005A=15`）；两侧 dump 各 2272 B
（`$0000-$07FF` + `$6AF4-$6BD3`），槽 1 基址 `Ens = $0410`、`EnsExtra = $6B04`。

驱动量逐帧**完全相同**（`status=2`、`speedY=$FC(-4)`、`speedSubPixelY=0`、`subPixelY=0`、
`accelY=0`、`Samus.y=176`），`data05` 交替 `0x1F`(bit6=0) / `0x5F`(bit6=1)：

| f | bit6 | emu y | rust y（旧形） |
|---|---|---|---|
| 969 | 0 | 196 | 196 |
| 970 | 1 | 196（整帧不动） | 192 |
| 971 | 0 | 192 | 188 |
| 972 | 1 | 192 | 184 |
| 973 | 0 | 188 | 180 |
| 974 | 1 | 188 | 176 |
| 975 | 0 | 184 | 172 |

窗口合计：emu 上升 12px/6 帧（每 2 帧 4px），rust 24px/6 帧（每帧 4px）。
**相位假象机械排除**：emu deltas `{0,-4,0,-4,0,-4}` 的任何整数平移都不等于 rust 的
`{-4,-4,-4,-4,-4,-4}`，不存在整数 k 使两序列对齐。

## 3. 修复后的观测

- `cases` 之外的单房复扫（`--captures 47,53,56,60,70,72,74,75,90,120`）：room 21 各采点
  全部 0px（`match 20 / mismatch 0 / error 2`，两 error 是采集点瞬态，非行为差异）。
- 全量 47 房复扫的净收益见 `data/status/room_sweep_0.json`（`note=bit6fix-area0`）。

## 4. 读法纪律（新增）

1. `ASL` 后面的 `BMI/BCS/BCC` 判的是**移位后的进位/符号**，不是原字节 bit7；写判据前先算一遍。
2. `data05 bit6` 是**每帧翻转**的伪随机门（`(x/16)^FrameCount`），凡"隔帧才动"的现象优先查它。
3. 同族例程有 7 处，改一处必须扫全族（本次扫出 2 处错、5 处对）。
4. 旧测试若用**不可达**的位值（如对本敌用 `0x80`）当规格，会把 bug 固化；新测试必须给出
   "改位后行为必变"的载荷断言。
