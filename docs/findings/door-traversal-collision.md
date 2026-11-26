# 门格碰撞瓦与到达性（#104 取证）

> 卡片：`tsk_01M8BQPY4TW3E4S1J4THHW`「无 poke 可玩穿门：门格碰撞瓦 `$A1` 写入链与到达性」。
> 取证日期：2026-11-26。HEAD = `d2fea892`（树里另有 #95 已验收未入库的 9 个改动，与本文无关）。
> 结论级别：A（字节/汇编行可复现）+ 测试（`cargo test -p game-core --lib door_traversal_tests / jump_apex_tests`）。

## 0. 结论（先看这四条）

1. **门格碰撞瓦是 `$A0`，不是 `$A1`**。房 5 左右门列都引 `_id_Structure03`，它的条目是
   `01 06`（metatile 编号 6），而 metatile 6 = `$A0`。卡面「`$A1` 写入链缺失」是伪前提。
2. **`$A1` 出现在两处，都与"门瓦写入链"无关**：
   - metatile 7 = `$A1`，只被 `Structure04` 引用（别的房间的门结构）；
   - 房间数据里的门**对象信息字节** `$A0 | doorType_Blue(=1) = $A1`，走 `SpawnDoorRoutine`，不写瓦。
3. **纯逻辑就能走到门格并触发进门**（无 poke）：从 `x=40,y=111` 按住 LEFT，第 17 帧
   `SamusDoorData($58)` = 1；再过 `SamusEnterDoor($8B13)` 一拍变 `$58`=`$11`、`$56`=`$80`（bit7）、
   `Samus.status`=5（`sa_Door`）。三条回归测试钉住这个链路。
4. **`$20`(32px) 不是跳跃顶点**，是"松开 JUMP"的截断阈值。按住 JUMP 时顶点由积分决定：
   **普跳 81 px / 35 帧、高跳 gear 108 px / 46 帧**。卡面「Rust 26px vs ROM ≈32px」的比较口径不成立。

## 1. 怎么复跑

```bash
# 门瓦 / 走到门格 / 进门触发（3 条）
cargo test -p game-core --lib world::tests

# 跳跃顶点（3 条）
cargo test -p game-core --lib jump_apex_tests
```

输出应为 `test result: ok. 3 passed` ×2（合计 6 条新增测试；`cargo test -q --workspace` 本机实测
`passed=1223 failed=0`，取数命令是把每个 `test result` 行的 passed/failed 相加）。

### 1.1 「无 poke 走到门格并触发进门」的可复跑命令（收窄口径）

```bash
cargo test -p game-core --lib world::tests -- --nocapture
```

期望输出（本机实跑，逐字）：

```
running 3 tests
test world::tests::room5_door_tiles_are_a0_not_a1 ... ok
test world::tests::walking_left_reaches_door_column_without_poke ... ok
test world::tests::door_tile_triggers_enter_door_state ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; ... ; 1109 filtered out
```

（末行的 `filtered out` 数量随其它测试增减，只有 `3 passed; 0 failed` 是本卡断言。）

三条各自钉住的量（断言失败会打印实际值）：

| 测试 | 断言 |
|---|---|
| `room5_door_tiles_are_a0_not_a1` | `$A0` 恰 24 个且只在 col 0-1 / 30-31 的 row 10-15；`$A1` 恰 0 个 |
| `walking_left_reaches_door_column_without_poke` | ≤25 帧内 `$58`(SamusDoorData) = **1**（左门 `cmp #$A0` 只加 1），且到门格时 **x ≤ 24** |
| `door_tile_triggers_enter_door_state` | 进门前 `$58` 先非 0；随后 `$56`(DoorEntryStatus) = **0x80**（bit7）、`Samus.status` = **5**(`sa_Door`)；房间号不变（本口径边界，见 §5.1） |

读 `$58` 用的是 `frame/memory.rs` 新增的 typed 访问器 `samus_door_data()`（与既有 `door_entry_status()` 同处、
同一写法；不裸用 `read_ram(ram_map::…)`，否则 `arch-coupling` 会把测试算成新的直接依赖）。

**这不是「完整穿门」**：房间号/MapPos 切换要走门对象链与滚动状态机，见 §5.1。命令里不含任何 `--poke`，
输入只有 `mem.joy1` 的 LEFT 位，走 `read_joy_pads` 的真采样路径。

- 门格三条在 `crates/game-core/src/world/tests.rs`（测试面文件，`no-raw-addr` 棘轮不计生产面；
  读值全走 typed 访问器：`mem.samus()`、`set_in_area`、`door_entry_status`、`read_joy_pads`）。
- 跳跃三条在 `crates/game-core/src/samus/physics.rs` 的 `#[cfg(test)] mod jump_apex_tests`。

## 2. 门格碰撞瓦的证据链

| # | 事实 | 指针 |
|---|---|---|
| 1 | metatile 6 = `$A0`×4，metatile 7 = `$A1`×4 | `refs/m1disasm/SRC/brinstar/metatiles.asm:34-37` |
| 2 | `Room05_{AREA}` 的左右门列都放 `_id_Structure03`（`($5<<4)+$0` 与 `($5<<4)+$F`） | `refs/m1disasm/SRC/brinstar/rooms.asm:113` |
| 3 | `Structure03_{AREA}` = `01 06 / 01 06 / 01 06 / FF` ⇒ metatile 6；`Structure04` = `01 07` ⇒ metatile 7 | `refs/m1disasm/SRC/brinstar/structures.asm:52-56` / `:58-62` |
| 4 | 门对象信息字节 `.byte re_Door, $A0 \| (0<<4) \| doorType_Blue`；`doorType_Blue = 1` | `refs/m1disasm/SRC/brinstar/rooms.asm:83` / `:121`；`refs/m1disasm/SRC/constants.asm:51` |
| 5 | 逐点读房间瓦的分派：`cmp #$4E` ⇒ 子弹打门；`cmp #$A0` ⇒ `SamusDoorData` 加 1；`cmp #$A1` ⇒ 加 2；`<$80` 固实、`$80-$9F` 可炸、`≥$A0` 可走 | `refs/m1disasm/SRC/prg7_engine.asm` 07:E7E6-07:E813 |

Rust 侧瓦判据（唯一实现）：`crates/game-core/src/world/room_ram.rs:51`
`tile_blocks_non_projectile(tile) = tile < 0xA0`。

### 2.1 房 5 实测瓦面（测试 `room5_door_tiles_are_a0_not_a1`）

`select_room_ram` + `init_tables` + `draw_room(5)` 后按 8 px 步进数 30×32 格：

| 瓦 | 计数 | 位置 |
|---|---|---|
| `$A0` | **24** | 左门列 col 0-1、右门列 col 30-31，各 row 10-15（2×6×2） |
| `$A1` | **0** | —— |
| `$FF`（空气） | 676 | 其余 |

与织码在可玩构建里的实测（`$4E`×6 + `$A0`×12 + `$A1`×0）差异在 `$4E`：那是 `WriteDoorBGTiles_Solid`
画出来的**可见门瓦**（`areas_common.asm` 的 `lda #$4E ; door tile`），属绘制层，不是碰撞判据
（`$4E < $A0` ⇒ 固实，所以"关着的门挡路"由它提供，而"能不能进门"由 `$A0/$A1` 提供）。
本仓 `draw_room` 走的是房间缓冲瓦面，不画 `$4E`，故测试里看到 24 个 `$A0`、0 个 `$4E`。

## 3. 走到门格 + 进门触发（测试证据）

`walking_left_reaches_door_column_without_poke`：`radiusY=16`（站立；球态 8，`SamusRoll` 起立
`lda Samus.radiusY / clc / adc #$08`，`prg7_engine.asm` 07:D0E3 段）、`radiusX=8`、`x=40,y=111`，
每帧只写 `JOY1_STATUS=0x02`(LEFT) 并走 `read_joy_pads` 采样 + `update_samus`：

```
断言的是「首个 $58≠0 的帧」，本机实测：f=17, $58 = 1, x = 19
（断言 f ≤ 25、$58 == 1、x ≤ 24；只断言「x 变小」不足以证明到门格，故断言点放在 $58 上）
```

`door_tile_triggers_enter_door_state`：同一路径，每帧在 `update_samus` 之后再调
`world::update::uw_samus_enter_door`（= asm `SamusEnterDoor ; 07:8B13`，`world/update.rs:144`）：

```
$56 = 0x80   (bit7 = SamusInDoor)
$58 = 0x11   ((1<<4) | 1，1=左门、1=进门那拍的 status)
status = 5   (sa_Door)
```

实现：`crates/game-core/src/samus/physics.rs:1395` `samus_enter_door`。

> 口径说明：卡面写的「`$56` 1→5」是**门滚动状态机**（`scroll_door`，`world/update.rs:24`）里的
> `DoorEntryStatus` 取值 1..4 + 出门态 5；`$8B13` 这一步写的是 bit7（`0x80`），两者不是同一个量。
> 见 §4 的未完成部分。

## 4. 跳跃高度：`$20` 的语义（测试 `jump_apex_tests`）

| 事实 | 指针 |
|---|---|
| `SetSamusJump`(07:CFC3)：`SamusJumpDsplcmnt←0`、`speedY←$FC`、`SamusAccelY←$18`（高跳 gear `$12`） | `refs/m1disasm/SRC/prg7_engine.asm:3435-3460` |
| `SamusJump`(07:D002)：上升中 `cmp #$20`，**只有 JUMP 未按住**才 `StopVertMovement` | 同上 `:3455-3481` |
| `MoveSamusUp`(07:E49B) 逐像素 `inc SamusJumpDsplcmnt`；`MoveSamusDown`(07:E4E9) 逐像素 `dec` | 同上 |
| 积分器 `VertAccelerate`(07:E37A)：`speed_f += accel` 进位入 `speed_i`，`subpix += speed_f` 进位入位移；上升钳位 -6 / 下降 +5 | `crates/game-core/src/samus/integrators.rs` `int_y`（证明级：2^32 穷举） |

按同一积分器逐帧算：

```
按住 JUMP（accel=$18）：顶点 81 px，35 帧到顶    ← 测试 holding_jump_reaches_81px_apex
高跳 gear（accel=$12）：顶点 108 px，46 帧       ← 测试 high_jump_gear_reaches_108px_apex
松开（dsplcmnt≥$20 截断）：32 px，第 9 帧        ← 测试 release_threshold_is_32px_not_apex
```

⇒ 63 px 高的门台，按住 JUMP 完全够得着；「跳不上去」若有，根因不在跳跃高度参数。

## 5. 未完成 / 遗留

### 5.1 端到端（开局 → 门格 → `$56` 1..5 → 房间号变化）**未达成**，两条路

我方判断与取舍（供 PMO 拍板）：

| 路 | 内容 | 代价 / 风险 |
|---|---|---|
| **A** | 把 #102 的 5 个 lane 提交（`9a9c6c6b`/`b5138b9e`/`69729fcb`/`a8086ed8`/`5b85aeb8`）pick 进 master，我再补端到端脚本（开局→门格→`$58`→`$56` 1..5→房间变化）与 `tools/` 回归命令 | master 上多 5 个提交；端到端仍可能被织码实测的「房 5 右门墙前天花板截断 34px」卡住——**真实房里是否可达尚未被证明** |
| **B** | 本卡按收窄口径收（门格可达 + 触发，已有 3 条断言测试），端到端另立卡 | 端到端风险留在后继卡；但本卡交付的每个量都有断言、可复跑 |

我倾向 **B**：本卡的 ② 在 master 上能证到的就是「走到门格 + 触发」，端到端依赖 lane 提交与真实载房，
属另一层；若走 A，我预计还要先解决织码报的几何截断（`probe_tile=4D`，房 5 右门墙前跳被截 34px）。

1. 端到端这条链要走
   `uw_all_doors`（UpdateAllDoors 的门对象链）+ `scroll_door`（滚动状态机）+ 载房。
   在合成中段的 setup 里（房间 240、无门对象初始化），加上 `uw_all_doors` 后 Samus 会在
   `x=28` 卡住不再前进（`$58` 始终 0），疑与门对象槽 / `$8B4F` 系状态不完整有关；
   要真复现需真实载房 + 完整帧循环。master 上没有 `--headless/--script`（只在 `lane/roll`：
   `9a9c6c6b`/`b5138b9e`/`69729fcb`/`a8086ed8`/`5b85aeb8`），所以"纯脚本走完整穿门"目前
   只能在 lane 上跑，主线不可复跑。
2. **`x=28` 卡住的定性**（工具口径 or 真缺陷）未完成：需要在真实载房上下文里重跑同一路径。
3. 本文只覆盖水平左门（`$A0`）与房 5；竖滚门（`$A1`，别的房间）未实测。

### 5.2 真实开局的端到端尝试（2026-11-26 实测；结论：不是"再加个脚本"能闭的）

环境（不动主线）：`git worktree add --detach /tmp/wt-e2e ed881af1`（`lane/roll` HEAD，已含
#102 的 5 个提交与 `--headless --script`），`CARGO_TARGET_DIR=/tmp/tgt-e2e cargo build --release
-p game-shell-pc`（11s），`assets/metroid.pack` 软链到主线。

复跑命令（脚本格式 `起始-结束 KEYS`，1-based，`+` 连接）：

```bash
/tmp/tgt-e2e/release/game-shell-pc --headless --script /tmp/e2e_door.script \
    --frames 1500 --state-out /tmp/e2e_door.csv
```

六个脚本家族（右行长走 2000 帧 / 左行长走 2000 帧 / 右行+每 200 帧跳 / 左行+跳 /
右行+开枪 / 每 60 帧连跳），每条 1500–2400 帧，逐帧读 `door_data($58)` 与 `door_entry($56)`：

| 脚本家族 | 末态 | `$58` 非零帧数 | `$56` 非零帧数 |
|---|---|---|---|
| 右行长走 | map 5/14，`x=0DC y=0C1`，`door_tiles=12/0` | **0** | **0** |
| 左行长走 | map 2/14，`x=004`，`door_tiles=0/0` | **0** | **0** |
| 右行 + 周期跳 | map 5/14，跳峰被 `4D` 截在 34px（f1410 到 `09F`） | **0** | **0** |
| 左行 + 跳 / 右行 + 开枪 / 连跳 | 同上量级 | **0** | **0** |

**为什么会 0**：房 5 的 12 个 `$A0` 门瓦在 rows 10–15（高台），Samus 在下层地板 `y=0C1`，
按住 A 的跳跃在 `09F`（34px）被 `$4D` 实心瓦挡住（**几何截断，不是积分错**——顶点已由
§4 三条测试钉死 81px/35 帧）。

把 Samus 用 `--poke` 摆到高台（`720:0x030E=0xD8`、`720:0x030D=0x68`）后：她落到 `x=0E4
y=071` 贴住门，但 **`probe_tile=4E`（关门瓦），不是 `$A0`** —— 门在高台上仍是**关着**的，
按 ROM 语义 `$4E` 不触发 `$58`（只有 `≥$A0` 才置 `SamusDoorData`）。
再在同一位置开枪（`735-745 B`）后重跑：`probe_tile` 仍是 `4E`、门槽 `0/0/2/0` 不变、
`$58` 仍 0 ⇒ **E2E 里"打门"没有把门打开**。
对照组：`cargo test -q -p game-core --lib weapon::projectile` → **13 passed / 0 failed**
（子弹命中 `$4E` → `samus_projectile_hit_door_or_statue` 的链在单测面是通的）。

**因此端到端未达成的原因是两层前置缺口，不是脚本写法**：
① 起始走廊的门在高台、必须先开门（纯走/跳到不了）；
② E2E 里开门链没跑起来（候选：门对象槽 `SpawnDoorRoutine` 在该房未完成、或 `--poke`
改 `status` 打乱了门状态机相位；未定性）。

### 5.3 正对照（受控证据）：链本身是通的，直到「请求载房」

> **口径声明：本节的站位是合成地图格（`set_room_number(0)`、MapPos 0/0），属诊断口径，
> 不构成可玩证据。** 它的作用是证明「门格命中 → 进门 → 出门 → 请求载房」这条链在
> 房 5 的真实瓦面上可以一路跑到底。

复跑：

```bash
cargo test -p game-core --lib world::tests::door_chain_runs_from_tile_to_load_request -- --nocapture
```

逐帧读数（实测，`crates/game-core/src/world/tests.rs` 的 4 条断言钉死）：

| 阶段 | 读数 |
|---|---|
| 按住 LEFT 走到左门格（无 poke） | **第 17 帧** `$58`(SamusDoorData)=**1**，`x`=19 |
| `uw_samus_enter_door`（SamusInDoor `$8B74`） | `$56`=`**0x80**`、`$57`=`**0x02**`（水平左滚）、`status`=`**5**`(sa_Door)、`$58`=`**0x11**`=(1<<4)\|1、`DoorDelay`=`**0x12**` |
| 出门序（20 帧） | `DoorDelay` 每帧 −2（0x12→0x02）、Samus 被移出门 `x` 19→**1**、`$56`→**0**、`status` 回落 |
| 收尾 | `RoomNumber`=`**$F0**`（载房请求哨兵），`MapPos` 未变 |

**为什么停在 `$F0`**：合成地图格没有可解析的邻房，载房窗无事可做；真实换房需要真实
`MapPos` + 载房窗（即 §5.2 里 E2E 走的路径，或 lane 上的 `--headless` 真帧循环）。

**与 §5.2 合起来的结论**：门链（`$A0` 命中 → `$58` → `$56` → status 5 → 出门 → 请求载房）
**是通的**；缺的两件在链外——①真实走廊里门在高台、纯走/跳到不了；②E2E 里开枪没把
`$4E` 开门（未定性候选，归「找真实可达门」探索卡）。

## 6. 与 #102 的关系

#102 的根因是 `SamusInDoor($8B74)` 把方向码写丢（`$56 = read($56)|0x80`，应为 `x|0x80`），
已修在 `lane/roll`（`9a9c6c6b`）。本文 §3 的测试在 master 上验证的是「置 `$58`→`$56` bit7→status 5」
这一步，方向码高值那部分要等 #102 的提交进主线才能一起验。
