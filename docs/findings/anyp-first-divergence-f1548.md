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
