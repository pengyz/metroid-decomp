# Norfair（area 1）46 房对拍台账：FC 相位纠偏前后

判据与工具见 `tools/movie_ref/room_sweep.py`：每房注入 `$005A=<room>` 让引擎自己载入
（不走路、与区无关），采集 7 个点（`arrived`/`pre`/`prev60`/`f60`/`prev120`/`prevend`/`f120`），
`ok` = 两腿跑完 + 每个采集点 diff==0 + 注入被接受（`$005A` 走完 `F0→…→FF` 且 `$33/$34`
落在该房流区间）+ `f900..f906` `$001E MainRoutine` 全 3。

## 1. 总量（`data/status/room_sweep_1.json`）

| 口径 | ok | mismatch | blocked | 备注 |
|---|---|---|---|---|
| 镜像修复后（`mirrorfix-area1`） | 13 | 27 | 6 | 名表镜像动态化前 |
| Crawler 按区 + Squeept 尾件后（`crawlerfix-area1`） | 14 | 29 | 3 | |
| 电梯换区帧会计后（`fcalign-area1`） | 21 | 22 | 3 | +7 房，零回退 |
| 移动串区基址 + 加速径两修后（`movstr-accel-area1`） | 34 | 9 | 3 | +13 房，零回退 |
| Crawler 静止出口动画节拍修正后（`animbeat-area1`） | 41 | 2 | 3 | +7 房，零回退 |
| **载房 BASE 第六点收窄后（`baserecal-area1`）** | **42** | **1** | 3 | 本次（room45 转绿，零回退） |

`movstr-accel-area1` 转绿 13 房（08/12/14/15/16/18/21/23/26/27/33/35/37/43），
两处机制：`$97EF` 只是 Brinstar 的串基址（`docs/findings/enemy-movement-ptr-truncation.md`）、
加速类 Δx/Δy 走积分件 + `$0406` 三名互斥（`docs/findings/enemy-acceleration-path.md`）。

`animbeat-area1`（1870.7 s）再 +7 房（10/11/22/24/30/34/40），`lost = []`。机制：
Crawler 静止出口的动画节拍必须取 `EnemyStatusPreAI`（1），写死 `2` 会让该槽动画相位
此后恒差 1 拍（每 3 帧 1 帧画出不同动画格）。逐帧取证见下一节。累计 21 → 41 ok。

机制：这些房里有「按 `FrameCount` 取模/移位做门」的敌人（Crawler 38 槽 / Swooper 族）。
换区迭代多烧 2 帧 ⇒ `FrameCount` 常差 2 ⇒ 位移节拍相位错 1 拍。
修复与证据见 `docs/findings/bank-switch-frame-accounting.md`。

## 1b. 剩余 2 房 + blocked 3 房（`animbeat-area1` 版）

- **room 19**（`prevend 220` / `f120 212`）：单槽 `slot5`（`type=00`、`status=01` Resting）
  字段面 14 处不同（`y/x/speedX/isHit/speedSubPixelX`、`EnsExtra.status/rY/rX/animFrame/
  animDelay/resetAnim/jumpDsplcmnt` 等），其余 11 槽全等；`ScrollX` 两腿同为 `238`、
  `FC` 同为 `164` ⇒ 不是帧相位，是该槽自身行为差（待解剖）。
- **room 45**（`prev60/f60≈11280`、`prevend/f120≈11011`）：**整屏**差 + `FC` 105/104、
  `ScrollX` 120/118 ⇒ rust 慢 1 帧；6 个空槽的 `data05` 也差（bit6 每帧翻转的相位面）。
  该房是 `DoorEntryStatus=1` 的自滚房（emu 载入尾段 f1600 `RoomNumber=45` → f1601 `F1`
  → f1602 `F3` → f1603 `FF`，`ScrollX` 每帧 +2），载房帧会计疑差 1 帧。
- blocked 3 房（28/31/32）仍为采集侧缺失。


### 1c. 剩余 2 房的逐帧取证（2026-11-22 收口）

**room 45 = 载房窗帧会计（cycle 模型刃口），不是行为差。**
逐帧探针（`/tmp/case-probe-r45step.txt`，1599–1625 每帧一采，`watch 002D FrameCount`）：

| f | 1599 | 1600 | 1601 | 1602 | 1603 | 1604 |
|---|---|---|---|---|---|---|
| emu `FrameCount` | 45 | 46 | **46** | 47 | 48 | 49 |
| rust `FrameCount` | 45 | 46 | 47 | **47** | **47** | 48 |
| emu `ScrollX` | 0 | 2 | 3 | 4 | 6 | 8 |
| rust `ScrollX` | 0 | 2 | 4 | 4 | 4 | 6 |

即：emu 在换房后**只冻结 1 帧**（1601），rust 冻结 **2 帧**（1601–1602 被吞，`--trace load`
直证 f1601/f1602 无 trace 行），此后 FC/ScrollX 恒差 1 帧 ⇒ 整屏 2px 相位差（11 280 px）。

对照审计（`tools/movie_ref/room_sweep.build_case` + `watch 002D FrameCount`，entry=1600）：

| room | emu 冻结帧 | rust 吞帧 | 一致？ |
|---|---|---|---|
| 22 | 1601,1602 | 1601,1602 | ✅ |
| 10 | 1601,1602,1603 | 1601,1602,1603 | ✅ |
| 45 | 1601 | 1601,1602 | ❌ 多烧 1 帧 |

`--trace load` 直读：room45 载房 `work_cycles=30766`，`dropped=(29000+30766)/29780=2`；
而 1 帧的上界是 `work ≤ 30060`——**超界 706 cycles**。room 45 是 `DoorEntryStatus=1`
的自滚房（`ScrollX` 每帧 +2），其载房工作量与滚动调用交织，`LOAD_BASE_CYCLES` 的整块
记账把「本可摊在多拍里的工作」算成一次超帧。**修法不是调常数**（那是阈值拟合），而是把
载房工作量按 `CheckUpdateNameTable` 的每次滚动调用记账（待做）。

**room 19 = Swooper01 的静息回切时机（`data05` bit5 置位晚）。**
- 槽级：`slot5`（Norfair `type=0x01` Swooper01）在 `f60` 两腿**逐字节相同**，`f120` 时
  emu 已 `status=1(Resting)` 且 `type` 读回 `0`，rust 仍 `status=2(Active)`、`type=1`。
- emu 逐帧（`watch 6B44 S5Status` + `watch 6B52 S5Type`）：`f1602 status=1/type=0` →
  `f1603 type=1` → **`f1642 status=2`（激活）** → **`f1720 status=1,type=0`（静息回切）**。
- rust 在 `f1720` 仍 Active，但其 `data05=0x2A` 已含 bit5（`0x20`）⇒
  `enemy_active` 的 `d & 0x20 != 0 ⇒ dec_status()` 下一帧才会回切 ⇒ **rust 的
  bit5 置位比 emu 晚**（`data05` 两腿差 `0x2B` vs `0x2A`，只差 bit0）。
- 结论：这不是 `enemy_active` 的分支错（分支已按 asm 接），而是 **bit5 的置位时机**
  （Swooper 距离阈值径）待对齐；下一轮在该径上做逐帧取证。


### 1d. room 45 收口：`LOAD_BASE_CYCLES` 第六点收窄（2026-11-22）

turn 的机制结论（§1c）指向「载房工作量记账过松」，但更精确的读法是**代价更低的那个
自由参数才是刃口**：`dropped = (BASE + work) / FRAME`，其中 `BASE`（载房以外的常规迭代
周期）在旧标定里只被夹在 `[28114, 29780)`，宽 1666c（5.6% 帧）。

新增第六点 emu 实录：**room45 `work=30766`，实测掉 1 帧**
（`(BASE + 30766) < 2×29780` ⇒ `BASE < 28794`），与 room20 的下界 `28114` 取交 ⇒
可行区间窄到 `[28114, 28794)`（680c），取中点 **28454**（旧值 29000 落在区间外）。

六点全部复核通过：room20 `work=31446`→2、room19 `36620`→2、room9 `43378`→2、
room23 `46456`→2、room8 `71946`→3、room45 `30766`→1，常规帧 `work=0`→0。
锚测 `world::room_draw::tests::load_window_model_calibrated` 的可行区间断言同步改成
`[28114, 28794)`（上界来源从「常规帧」换成 room45——它更紧）。

判据不变：room45 房扫 `prev60/f60/prevend/f120` 全部 0 px（修前 11280/11280/11011/11016）。

### 1e. room 19 的第一层：`EnemyMoveOnePixelLeft` 的换名表分支（2026-11-22）

room 19 的 `f60` 早已 0 px，残差从 `prevend/f120` 起（修前 220/212）。逐帧探针
（`case-probe-r19d05`，1690–1730 每帧一采，slot5 = Norfair type `$01` Swooper01）显示：
1690–1709 两腿逐字节相同，**1710 起 rust 的 `data05 bit0` 翻转**（`0B`→`0A`）且
`speedX` 由 `$FE`（−2）变 `$02`（+2）⇒ 走了「水平失败反弹」；emu 继续向左到 `x=0`。

asm 逐字（`refs/m1disasm/SRC/areas_common.asm` 的 `EnemyMoveOnePixelLeft ; $84A7`）：

- `dec x` 后 `x == radiusX` 这一径：`ScrollX == 0` ⇒ `inc x` 回滚失败；否则
  `jsr GetOtherNameTableIndex`（`$8563`，`A = (hi ^ PPUCTRL_ZP) & 1`，Z 置=与屏上名表相同）
  **非零（不同）⇒ `bne @success`**——允许跨名表继续左移。
- `x == 0` 那一径：只有 `ScrollDir >= 2` 才处理；`ScrollX == 0` ⇒ 失败；
  `GetOtherNameTableIndex == 0` ⇒ 失败（「试图在屏外换名表」）；否则
  `jsr SwitchEnemyNameTable`（`$855A`，`hi ^= 1`）。

rust 旧形两处都不合：`nx == rx` 恒回滚失败（不看 `ScrollX`/名表），`x == 0` 恒 `hi ^= 1`
且从不失败。room 19 命中的是前者。

修后探针 `1710/1711/1712 = 0 px`（此前 108/132 px），room19 复扫 `prevend/f120`
`220→217`、`212→209`——**只降了 3 px，说明反弹不再是这一臂，但还有别的机制**。
新增两条锚测（`left_move_wraps_when_scroll_x_and_nametable_differ`、
`left_move_rolls_back_when_scroll_x_is_zero`），红证：把成功径临时改成恒失败 ⇒ 前者 FAILED。

ROM 直读（A 级，PRG bank1，文件偏移 `16 + 0x4000 + (addr-0x8000)`）：

```
$84F4: BD 05 04 | 10 03 | FE 01 6B | 38 | 60      ; lda $0405,X / bpl→$84FC / inc $6B01,X
$8550: BD 05 04 | 10 03 | DE 01 6B | 38 | 60      ; 右移同形，dec $6B01,X
$855A: BD FB 6A | 49 01 | 9D FB 6A | 60           ; SwitchEnemyNameTable: hi ^= 1
$8563: BD FB 6A | 45 FF | 29 01 | 60              ; GetOtherNameTableIndex: (hi^PPUCTRL_ZP)&1
```

（`a75a142a` 的 commit message 里曾写过一行 `bank07:$84F4 附近 0A 10 0B`——**那是错的**，
全仓无此串，`$84F4` 也不在 bank7。以上四行是更正后的直读锚。）

> 【2026-11-23 更正】同一 commit 里我还「顺手」翻了左/右移 `jumpDsplcmnt` 记账的极性，
> 理由是误读了 `bpl`（`bpl` 在 N=0 时跳=bit7 **清**时跳过）。四件 asm 的真实极性是：
> 左/右 `bpl @endIf_E` ⇒ **bit7 置**才 inc/dec；上/下 `bmi @endIf_D` ⇒ **bit7 清**才
> inc/dec。原实现四件都对，那次翻转已回退（见下一条 commit）。

同批发现但**未修**（各自独立，留待单独一轮）：

- `EnemyMoveOnePixelRight` 的对应两处（`x == 0` 与 `x + radiusX == $FF`，判等方向与左移相反）
  仍是旧形，且界判写成了 `nx + rx == 0`（asm 是 `== $FF`）。
- `EnemyMoveOnePixelUp/Down` 的 `y == 0` / `y == $EF` 换名表径同样未移植。
- `CalculateFirstBGCollisionPoint ; 07:E90F` 的回绕修正敌径从未实现（Samus 侧在
  `crates/game-core/src/samus/physics.rs:40` 有）：Y 段借位 ⇒ `row −= $10` 且
  `ScrollDir & 2 == 0` 才 `inc hi`；X 段借位（distX 正）/ 越界（distX 负）⇒
  `ScrollDir & 2 != 0` 才 `inc hi`。试过一次性加进四个 `enemy_check_move_*` ⇒
  **分叉反而提前到 1713**，已整体回退，改为单独一轮逐步量测。

## 2. 剩余 9 房 mismatch 的分层画像（`movstr-accel-area1` 版，历史）

- **f120/prevend 才出现 57–220 px（6 房：10/11/19/24/30/40）**：`arrived`/`pre`/`f60` 全 clean
  ⇒ 载房后 ~120 帧内后出现的敌/弹/爆炸差，不是静态房数据。
- **f60 就出现（2 房：22/34）**：两房同签名——单槽 tough Crawler（`type=06`）的
  `EnsExtra+4 animDelay` emu `2` / rust `3`，该槽其余 **31 字节全等**（`Ens` 16 B 全等、
  `EnsExtra` 仅 delay 差），PNG 差 52/84 px 全落该精灵的 16×16 块。
  逐帧表（`probe-r22mid` 采点 1644–1675，步长 1）：两腿的 delay 周期同为 `3→2→1→3`、
  `animIndex`/`animFrame` 取值集合也相同（`6A/58`、`6B/59` 交替、`reset=69` 相同），
  但**相位恒差 1 拍**（`r.delay = (e.delay+1) mod 3`、`r.frame` = `e.frame` 的前一拍）
  ⇒ 每 3 帧里有 1 帧画出不同的动画格（≈50 px）。
  交叉帧对齐检验（`probe-r22phase`）：`emu[1660]` vs `rust[1659]/[1660]/[1661]` = 11320 /
  **52** / 11307 px ⇒ **不是采样缝**，同帧对齐是对的，属行为面 1 拍滞后。根因未定谳
  （候选：Crawler 在该区的「静止出口 `CommonJump_UpdateEnemyCommon_noMove` 的 A 值」
  与 `@exit` 的 `lda #$03` 谁在多算一拍；两腿 `animIndex` 全程相同排除了"beat 不同导致
  索引偏移"，故更像**调用次数**差）。
- **`scroll_drift`（1 房：45）**：`dx=±2` 族且取小后仍 11011–11280 px ⇒ 另有内容差。
- **blocked（28/31/32）**：`f120` 采集 blocked（模拟器侧采集缺失），需重采而非修代码。
- 继续解剖时优先取 `f60` 就先分叉的房（先 22/34），因为它们不依赖"后出现的敌人"。

## 2b. 上一版（22 房）的分层画像（历史）

- **`clean`→`sprite` 型（17 房）**：`arrived`/`pre`/`f60` 全 clean，`prevend`/`f120`
  才出现 57–697 px 精灵层差（8/9/10/11/12/14/16/18/19/21/22/24/26/27/30/33/35/37/40）。
  ⇒ 载房后 ~120 帧内「后出现的敌/弹/爆炸」的差异，不是房间静态数据差。
- **`scroll_drift` 型（3 房 + 2 房转 ok）**：`dx=±2` 成对口径族，取 `shift1` 后归零的房
  （13/29）判 ok；44 取小后 350 px、45 取小后仍 11280 px ⇒ **不是**单纯相位配对，另有内容差。
- **blocked（28/31/32）**：`f120` 采集 blocked（模拟器侧采集缺失），需重采而非修代码。

## 3. 已知的仪器边界（读数时必须记住）

- `prev120` 在所有房都是 `error`（lag 帧取不到参考图），按 `4db31751` **不参与判定**；
  台账里保留 `notes` 显式标注。
- 台账 `diff_pixels` 是**取小后**（`same` 与 `shift1` 两种配对取小）；判断"是否相位族"
  要看 `layers` 的 `scroll_drift` 标记，不要只看数值。
- 每轮 sweep 的产物文件名带独立 token（`_tmp_sweep_<pid>-<epoch>_a<area>_r<room>_f<frame>_<tag>`），
  **必须按台账 `captures[].file` 取 token**，用上一轮的 token 会配到已一致的一对（误判 0px）。

## 4. 下一步优先级（按机制归并，不逐房修）

1. 从剩余 22 房里取 `f60` 最早分叉者，按「敌槽级逐字节比对 → 机制锚 → 最小重现」解剖；
   Swooper0（24 槽/9 房）与 Multiviola（10 槽/6 房）是这一层最可能的公共机制。
2. room 45 的 11280 px 单独处理（量大且 layers 全段 scroll_drift ⇒ 可能是载入/滚动错位，
   而非敌逻辑）。
3. blocked 3 房重采（先确认模拟器侧采集稳定性，不修代码）。
