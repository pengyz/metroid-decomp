# any% 首分歧链：f1548 `animResetIndex` → f1549 `x+1px`

> 状态：**取证中**（根因未定谳）。本文只记录已证实的观测与已读的 asm，
> 不写"假设即结论"。数据来自 Mesen2（B 腿，裁定腿）与
> `inputlog movies/eien86-6725M.inputlog`（28 690 帧，与影片 100% 一致）。

## 1. 判据与窗口

- 输入：`movies/metroid-anyp-3666.bk2` 的真值 inputlog（`tools/movie_check.py` 100%）。
- 窗口：f0–4000（`tools/l4_diff_run.py run` + `watch_range`）。
- 逐字段首分歧：

| 字段 | 帧 | emu | rust |
|---|---|---|---|
| `room ($5A)` | 485 | `F2` | `F3` |
| `animResetIndex ($0305)` | **1548** | `0x55` | `0x00` |
| `x ($030E)` | 1549 | 191 | 192 |
| `hp ($0106)` | 2163 | 112 | 80 |
| `y ($030D)` | 2164 | 115 | 118 |
| `status ($0300)` | 2202 | 1 | 0 |

- `f485` 的 `room` 差是**已知模型缺口**（rust 尾游标 3 步模型跳过 `F2`，
  见 `room-tail-cursor-fix.md` §5），无可见像素差，不计入本次链。

## 2. f1548–f1551 逐帧（`$0305/$0306/$0304`、`$004D`、`$0313`、`$030E`）

```
 f    dirE dirR  subX_E subX_R   x_E x_R   resetE resetR  animE animR
1547   0    0     128    128     189  189     0      0        1     1
1548   1    1      80     80     190  190    0x55   0x00      5     5
1549   1    0      32    128     191  192     0x00   0x00      1     5
1550   0    0      80    128     193  193     0x00   0x00      5     5
1551   0    0     128    128     194  195     0x00   0x00      1     5
```

- 输入：f1547 = `02`(左)，f1548 起 = `01`(右)。
- **`dir` 两腿在 f1548 都是 1**；f1549 **rust 已翻 0、emu 仍 1**，emu 到 f1550 才翻。
- `x` 的差只在奇偶帧交替（1550/1552 又相等）⇒ 是**亚像素累加项**差，不是位置漂移。
- `animResetIndex` 在 f1548 两腿就不同（`0x55` vs `0x00`），`animIndex` 相同（5）。

## 3. 已读的 asm（A 级）

`SamusRun_SetAnim ; 07:CDBF`（`refs/m1disasm/SRC/prg7_engine.asm:3037-3052`）：

```asm
lda Joy1Status / and #BUTTON_UP / lsr lsr lsr / tax
lda RunAnimationTbl,x
cmp Samus.animResetIndex
beq RTS_CDBE            ; 相同 ⇒ 正常返回（后面 samL09~12 会跑，含 dir 翻转）
jsr SetSamusAnim
pla / pla               ; 丢弃返回地址
jmp SetSamusData_3FrameAnimDelay   ; 瞄准态变化 ⇒ 跳过 samL09~L12
```

`samL12`（`07:CD5F`）= `BitScan(Joy1Status)` → `cmp SamusDir` → 不同则 `sta SamusDir / jsr SetSamusRun`。

⇒ **"瞄准态变化"那一帧会短路掉同帧的 dir 翻转**。谁的 `animResetIndex` 变化、
在哪一帧变化，直接决定 `SamusDir` 何时翻——因此 f1548 的 `animResetIndex`
差（`0x55` vs `0x00`）是这条链的上游，`x+1px` 是它的下游表现。

## 4. 写者取证（半途，未定谳）

`ctwindow 1540 1552` + `ctwrite 0305/0306`（`analysis/traces/<case>.ctwrite.log`）
本次未捕获到 1548 的写点，却在日志里出现了 **f1555 起**的写入：

```
[ctwrite] f=1555 addr=305 val=00 pc=CF6E     ; SetSamusAnim 族
[ctwrite] f=1556 addr=305 val=37 pc=CF6E
[ctwrite] f=1562 addr=305 val=37 pc=CCB5     ; samL12 尾（sta SamusDir 邻域）
[ctwrite] f=1556 addr=306 val=38 pc=DCB0     ; UpdateObjAnim
```

两件事：①`0x37` 是 `ObjAnim_SamusRunPntUp - ObjectAnimIndexTbl`（瞄准上跑），
说明这段在 `Run`/`RunPntUp` 之间切换；②**`ctwindow` 的帧基与 trace 帧号不同**
（窗口 1540–1552 无输出、日志从 1555 起），需要先标定偏移，否则抓不到 1548。

## 5. 下一步（按顺序）

1. **标定 `ctwindow` 帧基**（与 `trace f` 的关系），然后用窗口覆盖 f1546–f1550，
   抓 `$0305` 的写者 PC——目标：确认 emu 的 `0x55` 由哪条 `sta` 写入。
2. 读该 PC 所在例程（`SetSamusAnim ; 07:CF6B` 族 / `SamusJump` 族），与 rust 的实现逐行对读。
3. 只有"哪一帧该写什么"定谳后才改代码；改完以 **f1548–f1560 逐帧 `$0305/$004D/$0313` 全等**
   作为验收，再看 `x` 是否归零。

## 6. 附带记录的仪器缺陷（已修）

`tools/l4_diff_run.py::emu_cache_hit` 只看锚（输入 sha），不看观察面；
改 `watch_range` 后可能复用旧 trace，得到 `w:{}` 空字典。已加 mtime 新鲜度闸
（`e90b7b91`），改动观察面自动重跑 Mesen。

## 7. 【2026-11-21 更正】本项不是 1px 残差，是整条死亡链的根因（待修）

按验收范式（`docs/spec/acceptance-paradigm.md`，用户 m41866 裁定：判据=能通关 + 人看录像无异常，
放弃逐帧强一致），本条 **不进 ratchet、不阻塞通关**，记 known-residual。

### 7.1 残缺链（已逐层实测）

1. 输入 `$14` 相同、`SamusDir` 相同，但 **samL12 的判据 `bit_scan(input) != SamusDir` 两侧结论相反**：
   emu 判「不等」，执行 `sta $4D<-01`、`jsr SetSamusRun($CC98)`（其内 `sta $0305<-0x37`）；
   rust 判「相等」，两者都不写，`$4D=00`、`$0305=00`。
2. 下一帧 `SamusRun_SetAnim($CDBF)` 拿 `RunAnimationTbl[0]=0x00` 与 `$0305` 比：
   emu 因 `0x37 != 0x00` 走 `SetSamusAnim` 清 0 并 `pla/pla` 短路（跳过 samL09-L12）；
   rust 因 `0x00 == 0x00` 不短路，照常走到 samL12 翻转。
3. `SetSamusRunAccel($CCB7)` 取 `RunAccelerationTbl[dir]`：`$30` / `$D0` 正负相反。
4. 亚像素累加后 x 差 1px（rust 1549 起与 emu 1550 对齐对比可见）。

### 7.2 已排除（逐字节核对，均一致）

- `SetSamusRun($CC98)` / `RunAnimationTbl($CCBE)` / `SetSamusRunAccel($CCB7)` /
  `RunAccelerationTbl($CCC0={30,D0})` —— rust 实现与 asm 逐件相同（`crates/game-core/src/samus/handler.rs:566`）；
- `SetSamusRun` 的调用次数：`cputrace CC98` 在窗口 1546-1550 内每转身帧**只命中一次**
  ⇒ 排除「被调两次」的假设（Mesen `f=1562 a=01`、`f=1564 a=00`）；
- 输入源：`.sched` 与 `inputlog` 由同一函数 `movie_import.read_inputlog` 生成（单源），
  且 `tools/phase_align_check.py` 实测 `$0014` 在 `rust f=N ≡ emu f=N+1` 下 **1326 帧零失配**。

### 7.3 未定（真缺口所在）

分歧点在 **`samL12` 的进入条件**（`bit_scan(input) != SamusDir`）或 **`samL11` 的提前返回**
（输入无 `RIGHT|LEFT` ⇒ `SetSamusStand` 后 return）。两侧该帧 `$14` 相同、`SamusDir` 相同，
故差只能来自「本帧是否走到 samL11/samL12」这层**帧内顺序**。

### 7.4 为什么难修（方法论结论，供后续复用时参考）

1. **无帧内事件面**：现有仪器只有帧末状态（ram/state 行）、写点/PC（`ctwrite`/`cputrace`，
   且 `ctwindow` 与 trace 帧号差 +15）、像素（最下游）。帧内顺序只能间接推断。
2. **每一环单独都"对"**：逐个函数对 asm 都能过，只有"这一帧进没进那段"是错的，逐行 diff 查不出。
3. **验证成本**：一次干净 Mesen 取证 ≈ 45s 且必须 `--fresh-emu`；一个假设两侧各跑一次。

### 7.5 若要让它变便宜（唯一可行方向）

做「帧内写序列 diff」仪器：两侧都记录每帧 `(pc, addr, val)` 写序列，按帧对齐后逐条 diff。
这样此类 bug 从"挖三轮"变成"看一次 diff"，也是让逐帧强一致重新可行的前提。**本轮不做。**

Evidence: 逐层实测见 §7.1-§7.4（`ctwrite` 四条写点 PC、`cputrace CC98` 两次命中、
`phase_align_check` 1326 帧零失配、`RunAccelerationTbl=$30/$D0` ROM 实读）。
Confidence: 85

### 7.6 更正：从 1px 到死亡链（2026-11-21）

当日全片复跑 + 逐字段对拍后，证据显示本项不是可挂账的 1px 残差，而是下文整条链的根：

1. `SetSamusRunAccel($CCB7)` 在 emu 的调用次数呈 `1,1,2,2,1,1,2,2…`（`ctwindow 1788 1797` + `cputrace CCB7`；
   双调用帧=该帧发生 dir 翻转：samL07 调一次、samL12 内 SetSamusRun 再调一次）。
2. 因 dir 翻转时点差一帧，两侧 `1/2 次` 的节奏错开，亚像素累加立刻出现恰好 `0x30`
   （`RunAccelerationTbl[0]` 的刻度）的差：f1790 起可见，f1794 显形为 x 差 2px。
3. 该差每几帧叠加一次：f4000 达 12px、f6000 达 100px 且 status 不同，f6263 rust 首次死亡。

结论：按验收三分法（渲染/功能/相位），本项属**功能异常**，必须修；先前挂 known-residual 系误判
（只看单帧 1px、未看它是种子）。修复目标=samL12 的进入条件（`bit_scan(input) != SamusDir`）
或 samL11 的提前返回，两者仍属帧内顺序，需 `--trace` 侧打印 `JOY1_STATUS/bit_scan/SAMUS_DIR` 定谳。

Evidence: `cputrace CCB7` 窗口 1788-1797 计数 1,1,2,2,1,1,2,2；`case-tmp-b7k` trace 上 f1790 起
subx 差恒 0x30、f1794 x 差 2px、f4000 差 12px、f6000 差 100px、f6263 死亡（`/tmp/stf.jsonl`）。
Confidence: 88

## 8. 修复后：首分歧推到 f2249（2026-11-21）

### 8.1 f1548 已修（commit `e03921bb`）
`SetSamusRun ; 07:CC98`（`:2824-2841`）的「转身过渡」支，asm 的 `inx` 在 `cmp PntUp`
**之前**且不回退 ⇒ 落该支时 **X 恒为 1**，`LCCBX: lda RunAnimationTbl,x` 写进
`animResetIndex` 的是 `RunPntUp=0x37`（本支只额外把 `animIndex` 设成 `Front`）。
旧 Rust 给 `x=0` ⇒ 写 `Run=0x00`。修复后 f1544/f1547/f1548/f1550 的
`animFrame/animDelay/animReset/animIndex` 与 emu 逐帧完全一致。

### 8.2 新首分歧 f2249（位置面 offset 0 对齐）
位置面对齐标定：`rust f=N` ↔ `emu f=N`（698/698 命中，f1500–2200），
`$0014` latch 面才是 `+1`。

| f | rust (status, spY, spX, y, x) | emu |
|---|---|---|
| 2248 | `00, 04, 08, C1, 73` | `00, 04, 08, C1, 73` |
| 2249 | `00, 04, F7, FE, C1, 72` | `00, 04, F7, BE, 75` |

### 8.3 emu 侧写者（`ctwrite`，Mesen 帧 = 脚本帧 +15）
```
f=2263 addr=30D val=C1/C0/BF pc=E49E    ← 同帧连减 3
f=2263 addr=30E val=73/74/75 pc=E695    ← 同帧连加 2
f=2264 addr=30D val=BE/BD/BC pc=E49E
f=2264 addr=30E val=75/76 pc=E695
```
两条 PC 落在：
- `$E49E` = `MoveSamusUp ; 07:E457` 的 `Lx155`（`CE 0D 03` = `dec Samus.y`）；
- `$E695` = `MoveSamusRight ; 07:E668` 的 `Lx183/Lx184`（`E6 51` = `inc SamusScrX`，
  `EE 0E 03` = `inc Samus.x`）。

⇒ emu 在该帧执行了 **MoveSamusUp(3 步) + MoveSamusRight(2 步)**，都是 1px 一循环；
Rust 同帧只做了 1px 且方向相反（x 73→72）。

### 8.4 未定谳
同一帧、同 `speedX=F7`、同 `status=0` 却出现「向上+向右」对「向左 1px」，
说明分歧在**更上游的方向/碰撞判定**（谁能走、往哪走），不在位移循环本身。
下一步：对读该位置两侧 RoomRAM 瓦片 + `MoveSamusLeft/Right` 的碰撞回退点。

### 8.5 决定性读数：emu 的 `ScrollX` 在 f2249 开始推进，Rust 的**冻结不动**

同一窗口（脚本帧 2244–2252）两侧 `$00FD ScrollX` / `$0051 SamusScrX`：

| f | emu ScrollX | rust ScrollX | emu SamusScrX | rust SamusScrX |
|---|---|---|---|---|
| 2244–2248 | `1EE4` | `E4` | `8F` | `00` |
| 2249 | `1EE6` | `E4` | `8F` | `00` |
| 2250 | `1EE7` | `E4` | `8F` | `00` |
| 2252 | `1EEA` | `E4` | `8F` | `00` |

⇒ **分叉的第一现场是"滚屏没发生"**：emu 从 f2249 起 ScrollX 逐帧 +1~+2，
Rust 的 `$FD` 恒为 `E4`。表观上的 `Samus.x +2 / y −3` 是 `MoveSamusRight`
（`$E695: inc SamusScrX / inc Samus.x`）与 `MoveSamusUp`（`$E49E: dec Samus.y`）
这些**滚动联动件**的产物，不是独立位移。

另注意：Rust 的 `$0051 SamusScrX` 恒 `00`（emu `8F`）——该字节在 Rust 侧可能
根本未建模（或语义不同），需一并核。

### 8.6 下一步
1. 读 `ScrollRight ; 07:E6D2` 与 `GetRoomNum ; 07:E720` 的**全部**出口条件，
   重点是 `ScrollBlockOnNameTable`（门挡路）与"到右边界"判；
2. 对读两侧 `$0049 ScrollDir` / `$004A TempScrollDir` / `$0056 DoorEntryStatus` /
   `$004E SamusDir` 在该帧的取值；
3. 顺带核 `$0051 SamusScrX` 在 Rust 侧是否有写点（当前观测恒 0）。
