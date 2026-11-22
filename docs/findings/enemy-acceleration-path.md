# 敌人"加速径"在移植里长期空转（2026-11-22）

## 1. 机制：两条互斥的位移来源

`L977B[type]`（`LoadTableAt977B`，`asl` 之后测 bit7 ⇒ 原字节 **bit6**）决定敌人怎么算出
本帧位移：

| bit6 | 位移来源 | asm | 说明 |
|---|---|---|---|
| 1 | 加速积分 | `EnemyGetDeltaY_UsingAcceleration ; $833F` / `EnemyGetDeltaX_UsingAcceleration ; $8395` | `accel*` 累进 `speedSubPixel*`/`speed*`，±14 钳位，再把小数累进位 |
| 0 | 移动指令串 | `EnemyGetDeltaY_SignMagSpeed ; $82xx` / `EnemyGetDeltaX_SignMagSpeed ; $8320` | 读 `EnemyMovementPtr` 指向的串（`SignMagSpeed` 记录） |

`EnemyMove ; $8058` 的调序恒为 **先 Y 后 X**，且 Y、X 各自独立判一次 bit6。
`EnemyGetDeltaX ; $8318` 的加速分支是 `jmp EnemyGetDeltaX_UsingAcceleration`——即
X 面用的也是积分件，而不是串的第二字节。

## 2. 移植侧的两个缺口（本次修复）

`crates/game-core/src/enemy/common.rs::enemy_get_delta_xy` 是纯函数（只吃字节，不持
`Manager`），它的加速分支做了两件错事：

1. **算了不写**：局部 `sub`/`sp` 算完 `let _ = (sub, sp);`，只把 `sp` 当 Δy 返回 ⇒
   `speedY`/`speedSubPixelY` 永不更新 ⇒ **加速度等于不存在**，加速类敌人恒速。
2. **Δx 恒 0**：加速分支直接 `return (sp, 0, …)`，非 0 的 `speedX` 一律丢弃。

而同一份积分语义其实早已端口（`motion::enemy_get_delta_y/x_using_acceleration`，
PipeBug/Squeept 在用），只是**通用移动路径从未调用它**。

第三个坑在回写顺序上：`$0406` 一字节三名
（`Ens.0.chunkyExplosionTimer` / `Ens.0.movementInstrIndex` / `Ens.0.speedSubPixelY`，
`refs/m1disasm/out/M1_NES_NTSC.sym:11118-11120`）。串径用它存指令索引、加速径用它
存 Y 亚像素，**原版互斥**。旧形无条件 `write_ram(EN_INSTR, ninstr)`，加速径把刚积分
出来的亚像素覆盖成 0。

修法：`enemy_delta_for_move` 在加速类时改调两个积分件（先 Y 后 X），并把 `$0406` 的
指令索引回写限制在串径。

## 3. 实测（Norfair room 19 slot5 = Swooper01）

`L977B[1] = $4C` ⇒ 加速类；`data05 = $0B`；Swoop 径写 `accelY = $80`（朝天花板）。

| 字段 | emu | rust（修前） | rust（修后） |
|---|---|---|---|
| `Ens.y` | `7D` | `8B` | `7D` |
| `Ens.speedY` | `07` | `0C` | `07` |
| `Ens+6`（`speedSubPixelY`） | `80` | `00` | `80` |
| `Ens.x` | `3A` | `4C`（Δx 修前） | `3A` |
| `EnsExtra.subPixelY` | `80` | `00` | `80` |
| `EnsExtra.jumpDsplcmnt` | `AB` | `9D` | `AB` |

修后该槽 `Ens` 16 B + `EnsExtra` 16 B **逐字节相同**；房扫 `prev60`/`f60` 由 297 px
降到 **0** px。

## 4. 遗留

- `prevend`/`f120` 仍有 220/212 px（另一机制，未定谳）。
- 同族待查：`EnemyGetDeltaY_SignMagSpeed` 的 `$00`/`$01` 暂存语义、以及"加速径敌人
  的 `movementInstrIndex` 与 `speedSubPixelY` 复用同一字节"在存档/快照往返里的表达
  （`TypedRestoreManifest` 目前只按一个名字声明 `$0406`）。

Evidence: asm `areas_common.asm:803 EnemyGetDeltaX` / `:833F` / `:8395`；`sym:11118-11120`
三名同址；room 19 slot5 修前修后 32 字节对读；房扫 297→0 px。
Confidence: 94
