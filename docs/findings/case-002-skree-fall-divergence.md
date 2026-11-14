# case-002 万像素级分歧的根因：敌 Skree 下落速度（2026-11-15）

> 结论先行：`case-002-door-blue-interaction` 的 6 个失配格（203 / 10907 / 9806 / 9806 / 13450 / 13456 px）
> **全部由一颗 Skree（En 槽 2）的下落速度差派生**，与「门进入滚动」无关。此前把该案归因为
> 「过门后滚动时序」的判断作废。

## 1. 取证方法（可复用）

`analysis/traces/<case>.frames.jsonl` 是金腿（Mesen2）逐帧快照，列名由 case 文件的 `watch` 行决定；
`shots_rust/<case>.state.jsonl` 是锈腿整面内存编码，取值规则见
`crates/game-core/src/memmap/frame_face.rs`：`zp` 覆盖 `$0000-$00FF`、`iram` 覆盖 `$0100-$07FF`、
`wram` 覆盖 `$6000-$7FFF`（`$6AF4` 的 EnExtra 面走 wram）。

帧对齐用 **offset 0**（锈腿 `f=N` 与金腿 `f=N` 同帧）：偏移 ±1 会因 `data05` bit6 每帧翻转
（30FPS 门）产生数百帧假分歧，是本案最容易踩的坑。

## 2. 首因定位

`watch` 了 `0420/0421/0425/0426/0427/0428`（En 槽 2 的 y / x / data05 / 指令索引 / 子像素）
与 `0049/004a/004f/0050/0051/00fd`（ScrollDir / TempScrollDir / MapPosY / MapPosX / SamusScrX / ScrollX）：

| 帧 | 金腿 y | 锈腿 y | 备注 |
|---|---|---|---|
| 1050–1075 | 逐字节相同 | 逐字节相同 | 含 `data05` 的 `$13/$53` bit6 交替 |
| 1076 | `$44` | `$42` | **首个分歧**，此后不再收敛 |
| 1110 | `$A2` | `$94` | 差 14 px |
| 1188 | `$C4`（爆体清槽） | 仍在屏上 | 金腿已走完 `instr >= $11` |

金腿该敌自 `f927` 出生（Resting，`y=$38 x=$8C`），`f1066` 转 Active，`f1188` 爆体消失；
锈腿同一路直到 `f1240` 仍停在 Active，位置恒定在 `(y=$88, x=$90)`。

## 3. 连锁如何放大成万像素

1. 锈腿下落慢 ⇒ `EnemyDistanceToSamusThreshold` 的「近距」位（`data05` bit3）在 `f1082` 被清，
   金腿保持置位；
2. 金腿的 Skree 在 `f1113` 先压到 Samus，触发受击击退（`speedX=$FE` 即 -2、`speedY=$FD` 即 -3）；
   锈腿到 `f1119` 才被压到，且击退方向相反（`speedX=+1`）；
3. 两侧 Samus 从此一条向左、一条向右走（`MoveSamusLeft` / `MoveSamusRight` 每帧 ±1 px，
   `SamusScrX` 触到 `$71` / `$8F` 后各自触发滚屏）；
4. `f1160` 金腿 `x=$4a scrx=$70 ScrollX=$da ScrollDir=2 MapPos=(3,14)`，
   锈腿 `x=$a3 scrx=$8f ScrollX=$14 ScrollDir=3 MapPos=(5,14)`；
   锈腿因此向右走到一扇门前并把它打开（`Door` 槽 10 `status=2`），金腿全门为 0。

## 4. 字段偏移的权威来源（纠错）

`refs/m1disasm/out/M1_NES_NTSC.sym` 是**汇编产物的符号表**，直接给出每个字段的地址，
比读 `.struct` 声明可靠：`En` 的 `$0406` 一个字节被三个名字共用
（`chunkyExplosionTimer` / `movementInstrIndex` / `speedSubPixelY`），`$0408` 才是 `movementIndex`，
`$0409` 是 `delay`。EnExtra 侧 `status=$6AF4`、`hi=$6AFB`、`accelY=$6AFE`、`type=$6B02`、`data1F=$6B03`。
锈腿现有常量与之一致，无需改动。

## 5. 仍未定谳的一点

Skree 的下落由 `EnemyMove`（`refs/m1disasm/SRC/areas_common.asm:111`）驱动：
`EnemyGetDeltaY` 给出这一帧要走的像素数，再逐像素调用 `EnemyMoveOnePixelDown`。
金腿的 `movementInstrIndex` 从 0 递增到 `$0F`（落地）再 `$11`（爆体），锈腿慢一步。
**下一步**：在锈腿的 `skree_ai` 里打印「使用的串指针 / 串内偏移 / 本帧 Δy」，与金腿的
`movementInstrIndex` 逐帧对照，判定是串选错（`movementIndex`）还是 `$FC/$FE` 的
「重复上一条」语义差一帧——两者都会表现为「Δy 恒小一半」。

## 6. 口径

本文件只记录已证实的因果链与字段事实；§5 是待办，不是结论。

## 7. 补充：指令节拍只有一半（2026-11-15 补测）

给 `skree_ai` 临时加打印（`DSH_SK_DBG` 门控，提交前已删除）后，锈腿侧读到的
`movementIndex` 是 **7**（`EnemyMovementPtrs[7] = $9891/$98A5`，与 mov7 串
`05 0a 07 9a 05 ba 04 da fd ff 01 01 ff 01 09 ff 04 22 01 42 …` 一致），说明串没选错。

问题在节拍：同一条 `SignMagSpeed` 指令在锈腿要占 **4 个移动帧**才前进两字节，
金腿只占 **2 个**。这正好等于观察到的 y 速度差（金腿每 2 帧 +6 px，锈腿 +2~4 px），
也解释了指令索引 `$0406` 推进速度只有金腿的一半。

`areas_common.asm` 的 `EnemyGetDeltaY_SignMagSpeed`（`:622-646`）语义是：
`duration − delay == 0` 时置 `delay=0`、`instr += 2` 并**在同一帧继续读下一条**；
否则 `delay += 1` 并发出本帧 Δ。下一步只需把这段节拍与锈腿逐行对齐，
即可判定是「前进那帧少发一次 Δ」还是「比较用了 `>=` 而不是相等」。
