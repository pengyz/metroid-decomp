# 换区帧会计：电梯到达迭代只烧 3 帧（`FrameCount` 相位纠偏）

结论：`ElevatorD8BF ; 07:D8BF`（换区核心）所在的那一次 `MainLoop` 迭代，
真实等待是 **3 帧**（`ScreenNmiOff` 1 + `WaitNMipPass_` 1 + `ScreenOn` 1），
实测与 asm 两侧一致。旧 Rust 模型把电梯侧再烧 2 帧（两处 `iter_wait`）叠在
`init_bank2` 已挂的 3 门之上 ⇒ 一次换区多烧 2 帧，`FrameCount($2D)` 从此落后 2，
凡是「每帧按 `FrameCount` 取模/移位做门」的敌人（Crawler、Swooper 族）全部错相位。

## 1. 症状与起点（room 38，tough Crawler）

Norfair room 38 的 `f120` 采集点：两腿 `Ens`/`EnsExtra` 只差 2 px（emu `y=0x86` vs Rust `0x88`，
`EnsExtra+13 jumpDsplcmnt` 0xD2 vs 0xD0），`f60` 却逐字节相同。
60 帧内 emu 下移 44 px、Rust 46 px ⇒ 同为 6/8 帧节拍，但**节拍相位**不同。

机制锚（A 级）：`ShouldCrawlerMove ; 07:F852`，Rust 侧
`crates/game-core/src/enemy/crawler.rs::crawler_move_gate(x, fc)`：

```
((x>>3) + ((x>>2)&1) + fc) >> 1 & 0x03      // gate==0 ⇒ 本帧不移动
```

`fc` 差 2 ⇒ `(…+fc)>>1` 相位差 1 ⇒ 60 帧窗内移动帧数差 1–2，与实测 2 px 吻合。

## 2. `FrameCount` 偏移落在换区窗（机械定位）

探针 `watch 002D FrameCount` + 每 40 帧采集（`case-probe-fc-north38`）：
`f320…f1200` 差恒 0，**`f1240` 起恒 2**。逐帧带（capture 1198–1215）看得更清楚：

| 帧 | emu FC | rust FC |
|---|---|---|
| 1204 | 164 | 164 |
| 1206 | 166 | 165 |
| 1207 | 166 | 166 |
| 1208 | 166 | 167 |
| 1209 | 167 | 169 |
| 1215+ | — | 恒差 2 |

emu 在 `f1206/1207/1208` 冻结 3 帧后 `f1209` 继续；同一窗口 `InArea=0x11` 两侧一致、
`MainRoutine` 恒 3、`RoomNumber` 恒 `FF`。

emu 的 `FrameCount` 冻结窗全集（1720 帧用例）：`f2`、`f4-6`、`f311-313`、`f352-358`、
`f1206-1208`（=本次换区）、`f1601-1602`（=载房尾段）。前四组在标题/入场段。

## 3. asm 事实（A 级）

- `MainLoop` 迭代序：`CheckBankSwitch($C4DE)` → `UpdateTimer($C266)` →
  `GoMainRoutine($C114)` → **`inc FrameCount`（`07:C0C5`）** → `sta NMIStatus(0)` →
  自旋等 NMI → `RandomNumbers`。⇒ 迭代体内的每次等待都发生在 `inc FrameCount` **之前**，
  故一次换区在 FC 上表现为「冻结若干帧 + 尾 inc 一次」。
- `WaitNMIPass ; 07:C42C` / `WaitNMIPass_ ; 07:C43F`：`jsr ClearNMIStat`（`$1A←0`）
  后自旋等 `NMIStatus != 0` ⇒ 最多消耗 1 帧。
- `ScreenOff ; 07:C439` / `ScreenOn ; 07:C447` → `WriteAndWait ; 07:C43D`：同样 1 帧等待。
- `ScreenNmiOff ; 07:C45D` = `WriteAndWait` 后清 `PPUCTRL` NMI 位（1 帧）；`NMIOn ; 07:C487`
  只等 vblank 收尾再开 NMI（不整帧）。
- `ElevatorD8BF`（`prg7_engine.asm:5005-5056`）：`jsr IsEngineRunning`（内联
  `SwitchBank`→`CheckBankSwitch`→`GoBankInit`→`InitBankN`，同迭代完成）→ `PaletteToggle`
  → `sta PaletteDataPending` → **`jsr WaitNMIPass_`（:5030）** → `SelectSamusPalette`/
  `StartMusic` → **`jsr ScreenOn`（:5036）** → `CopyAreaPointers`/`DestroyEnemies`/淡入。
- `init_bank2 ; 07:C583`：`GameMode←0` / `jsr ScreenNmiOff`（1 帧）/ `jsr InitNorfairGFX`
  （纯 CPU 装载）/ `jmp NMIOn`。

⇒ 换区迭代总等待 = 1（ScreenNmiOff）+ 1（WaitNMIPass_）+ 1（ScreenOn）= **3**，
与 emu 实测冻结 3 帧逐帧对齐。

## 4. 两个对照假设（实验判决）

| 假设 | 改动 | 结果 |
|---|---|---|
| A | `init_bank2` 的 `iter_bogged(2)` → `0` | FC 对齐、room38 34/34 全 0px；但 `case-password-justin` 的 `f1197 last-dark` 由 match 变 **11431 px** ⇒ **否决** |
| B（采用） | 保留 `init_bank2` 的 3 门，删电梯到达的 2 处 `iter_wait` | FC 对齐（仅 1206-1208 有 −1 帧窗内相位）、room38 **34/34**、`case-password-justin` **12/14**（唯一失配=f700=23/f1100=42，即既有 `oracle-present-pipeline-lag` 挂账两格）、矩阵 **166/172 与基线逐格一致** |

为什么 A 会翻车：`cases/case-password-justin.txt:4,:33` 注明该场景**游戏内 spawn 在
Norfair**（`watch 0074 InArea`，`InArea=0x11`）⇒ 密码进场同样走 `init_bank2`，
它的暗窗 `f1191–f1197`（7 帧黑）正是靠这 2 门拟合出来的宽度。删掉后 rust 提前 2 帧点亮。

⇒ 两个场景共用 `init_bank2`，但**只有电梯径**能承受减 2：电梯径里 `ScreenNmiOff` 的等待
与紧随其后的 `WaitNMIPass_`/`ScreenOn` 落在同一迭代的连续 NMI 窗口内，
门队列与实际等待不是 1:1；而密码进场径的等待是分开的迭代预算。

## 5. 已知缺口（未闭合，留判据）

1. 换区窗内 `f1206-f1208` 仍有 **1 帧相位差**（rust 领先 1），窗后归零——属"同窗多等待如何折叠"
   的细粒度问题，需要 emu 侧 `cwp` 级写点/PC 取证才能再细化。
2. `f1601-f1602`（载房尾段）的 2 帧冻结尚未对齐（不影响 FC 常差，因在尾部）。
3. `init_bank3/4/5` 的 `iter_bogged(2)` 仍标 C 级（Tourian 载 10 件，未对拍校准）。

## 6. 可证伪判据（后继改动必须满足）

- `case-probe-fc-north38` 34/34 全 0px（FC 与像素双面）。
- `case-password-justin` 不低于 12/14，且失配只能是 f700/f1100 两格。
- 矩阵 21 案 / 172 格 ≥ 166 match（不得出现新回退）。

## 7. 读法纪律

- 「一次等待 = 一帧」只在**迭代边界处**成立；同一迭代内连续多次 `ClearNMIStat` 自旋，
  帧数由 NMI 到达时刻决定，不能按调用次数累加。
- `FrameCount` 是**下游门控变量**：发现某敌人"节拍相位"错时，先查 FC 常差，再查敌人逻辑。
- 拟合常数（`iter_bogged`/`iter_wait`）改动必须在 findings 留可证伪判据，并跑"两个互斥用例"
  才会暴露单侧拟合（本案 A 假设就是被 `case-password-justin` 拦下的）。
