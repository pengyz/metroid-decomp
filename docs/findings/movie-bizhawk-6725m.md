# 发现：BizHawk(quickerNES) 加载 TASVideos 6725M 取证 + Rust 腿前 1000 帧对齐

- 日期: 2026-11-18
- 结论（一句话）：本条录像在本机 **BizHawk 2.10 无头（Xvfb + mono 6.8）可跑通**，核心为
  **quickerNES**（= 录像头的 `Core=quickerNES`，同源）；逐帧 RAM 轨迹已导出
  `analysis/traces/bizhawk-6725M.jsonl`（1201 行，`f=0..1200`）；前 1000 帧与 Rust 腿
  **967 个可比帧中 957 帧四字段全同、10 帧失配**，失配全部落在**上电窗口**与**room 过渡相位**
  （±1~2 帧时序差 + `$5A=09` 中间值缺失），**无 health 面失配**。
- 附带证伪：`tools/movie_import.py:128` 的 bk2 输入位序与 bk2 自带 `LogKey` 不匹配
  （方向键 4 位错位），**既有 `movies/eien86-6725M.inputlog` 的方向键是错的**（§7，已量化影响）。
- 证据分级：本文所有 RAM 地址均给仓库锚（文件:行号）；所有「一致/不一致」结论均附实测命令与数字；
  **未执行的事不写进结论**（见 §9 未做清单）。

## 1. 被取证对象与环境

| 项 | 值 | 证据 |
|---|---|---|
| ROM | `rom/Metroid (USA).nes`，131088 B | `ls -l` |
| ROM SHA1（整文件） | `ECF39EC5A33E6A6F832F03E8FFC61C5D53F4F90B` | 运行日志第 4 行 / 录像头 `SHA1` |
| 录像 | `/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2`（TASVideos 6725M） | 文件存在；header `Core=quickerNES` `BoardName=mmc1` `Frames=28408` |
| BizHawk | 2.10，`/tmp/bh`，mono 6.8 | `strings EmuHawk.exe` / `client.getversion()`=2.10 |
| 显示 | Xvfb（`xvfb-run -a`），无窗管 | 见 §3 |

## 2. 核心选择（结论：quickerNES，A 级证据）

三条独立证据互相印证：

1. **配置键**：`/tmp/bh/config.ini:11` → `"NES": "quickerNES"`（同文件 `:2` 起为 `"PreferredCores"` 块）。
   这是 BizHawk 2.10 指定 NES 核心的键（`--config <ini>` 可改；本次使用默认 config）。
2. **本构建只有 quickerNES 的 NES 原生件**：
   - `/tmp/bh/dll/libquicknes.so`（285112 B，ELF 共享库）；
   - `/tmp/bh/dll/` 中与 NES 相关的其它件：**无** `libneshawk*.so`、**无** NES 的 `*.wbx.zst`
     （该目录只有 `bsnes.wbx.zst`/`libsnes.wbx.zst`/`snes9x.wbx.zst` 三个 SNES 件，及 `libMSXHawk.so`）；
   - `strings -a /tmp/bh/dll/BizHawk.Emulation.Cores.dll` 命中：
     `1.0.0,https://github.com/SergioMartin86/quickerNES`、
     `QuickerNESInternalControllerTypeEnumeration`、`BizHawk.Emulation.Cores.Consoles.Nintendo.QuickNES`。
     （同程序集另有 `BizHawk.Emulation.Cores.Nintendo.NES`/`SubNESHawk` 类字符串，但本构建无对应
     独立原生件、且 `PreferredCores` 未选它——不作为本次核心。）
3. **运行时日志自证**（每次运行都打印，且出现两遍——BizHawk 加载流程里的二次 core 初始化）：
   ```
   ECF39EC5A33E6A6F832F03E8FFC61C5D53F4F90B
   quickerNES: Booted with Mapper #1 "mmc1"
   Hash for BootGod: FDBFC7871962F72A1EF57E5A7E456164FB93430B
   BootGod entry found: Metroid
   Core reported BoardID: "mmc1"
   Read a zipstate of version 1.0.2
   ```
   `quickerNES: Booted with Mapper #1 "mmc1"` 是 quickerNES 分支（SergioMartin86 fork）自己的启动行，
   与本录像 header 的 `Core=quickerNES` 同名同源；`BoardID: mmc1` 与 header `BoardName=mmc1` 一致。

**影响面**：因此本次「作者核心轨迹」= quickerNES 轨迹，与录像录制时的核心**同一实现**（不存在
NesHawk/QuickNES 上游版的核心不完全对齐问题）。唯一残留不确定性是同一 fork 的**版本差异**
（录像未记录 quickerNES commit），本报告不声称「逐指令同版本」。

## 3. 无头运行（结论：能跑；但有非致命 X11 报错 + 退出不干净）

- **必需环境变量**：`ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy`
  （缺省时 OpenAL 初始化会卡住主线程——症状是进程不退出、无 Lua 输出、strace 显示 X socket 忙轮询）。
- **非致命报错**（每次启动 2 次，不影响取证）：
  ```
  X11 Error encountered:
    Error: BadMatch (invalid parameter attributes)
    Request:     12 (0)
    Resource ID: 0x20007A
    ...
    Control:     BizHawk.Bizware.Graphics.Controls.OpenGLControl
    at BizHawk.Client.EmuHawk.MainForm.SetStatusBar ()
    at BizHawk.Client.EmuHawk.MainForm.MainForm_Load (...)
    at BizHawk.Client.EmuHawk.Program.SubMain (System.String[] args)
  ```
  实证不影响：Lua 全片逐帧执行、录像正常播放、轨迹文件正常写出并 close。
- **退出不干净**：`client.exit()` 之后日志出现
  `BizHawk has completed its shutdown routines, killing process...` 但进程不退出，
  被外层 `timeout` 杀掉（shell `exit=137`）。**轨迹在杀之前已 `flush`+`close` 完整落盘**
  （见 §5 的确定性复现：两次运行产物逐字节相同）。

  > 【2026-11-18 更正】「被外层 timeout 杀掉」是当时的误判。用
  > `tools/movie_ref/run_bizhawk_trace.sh` 做 20 帧计时实测：**整体 wall 1.9 s**（`timeout 300`
  > 远未到期）却已 `exit=137`，且 stderr 紧接 `killing process...` 打印 shell 的 `Killed`
  > ⇒ 137 是 **BizHawk 自己在 shutdown routines 末尾对本进程 SIGKILL**（`killing process` 就是
  > 它的原话），不是 timeout 干的。正确表述：**进程「自杀式」退出（137）+ 产物完整**；
  > 外层 `timeout` 只是保险，不是退出原因。判定成功永远看产物行数，不看退出码。
- **`--userdata`/`client.getuserdata` 在本构建上不可用**：`--help` 确认参数格式为
  `--userdata k1:v1;k2:v2`，但带 `--userdata frames:50` 运行时 180 s 内既不产轨迹也不退出
  （挂死，被 timeout 杀）；改用环境变量 `BH_FRAMES` 后正常。根因未定位（疑与 `comm.socket*` IPC
  绑定，本次不需要 socket）。**结论：帧数用 `BH_FRAMES` 控制，不用 `--userdata`。**

## 4. Lua 轨迹脚本与字段地址锚

脚本：`tools/movie_ref/bizhawk_ref.lua`（逐帧写一行 JSON；`memory.read_u8(addr,"RAM")`，
BizHawk 的 `RAM` 域地址 == NES 绝对地址——`$0300/$005A/$0014` 读出合理值即证）。

| 字段 | 地址 | 仓库锚（禁止猜） |
|---|---|---|
| `health_lo` | `$0106` | `crates/game-core/src/ram_map.rs:169` `SAMUS_HEALTH: u16 = 0x0106`（BCD，+1=高位） |
| `health_hi` | `$0107` | 同上（`$0106+1`） |
| `status` | `$0300` | `crates/game-core/src/ram_map.rs:223` `SAMUS_STATUS: u16 = 0x0300` |
| `room` | `$005A` | `crates/game-core/src/ram_map.rs:216` `ROOM_NUMBER: u16 = 0x005A`（`$FF`=undefined） |
| `scrolldir` | `$0049` | `crates/game-core/src/ram_map.rs:59` `SCROLL_DIR: u16 = 0x0049` |
| `joy1_latch`/`joy2_latch` | `$0014`/`$0015` | `crates/game-core/src/ram_map.rs:70,73` `JOY1_STATUS`/`JOY2_STATUS`（ROM 读手柄后的锁存） |
| `p1`/`p2` | 非 RAM | `movie.getinput(f)`（BizHawk 2.10：已加载录像第 f 帧输入），按 A=bit7…Right=bit0 折算（`crates/game-core/src/bin/rust_screenshot.rs:25-27` 同一位形） |

字段语义补充（取证观察，非推断）：`$0014` 比录像输入**晚一帧**锁存——录像 f13 按 Start（`p1=0x10`），
`$14` 在 f14 才出现 `0x10`（轨迹 f13/f14 行可查）。这是 ROM 读手柄时序，不是采样错位。

## 5. 轨迹产物

| 项 | 值 |
|---|---|
| 文件 | `analysis/traces/bizhawk-6725M.jsonl`（`analysis/` 不入库） |
| 行数 | **1201**（`f=0..1200`，每行一帧） |
| 每行字段 | `f, health_lo, health_hi, status, room, scrolldir, p1, p2, joy1_latch, joy2_latch` |
| 确定性 | 同一命令跑两次，产物 **逐字节相同**（`diff -q` 通过） |
| 全片导出 | 脚本支持 `BH_FRAMES=28408`（粗测 ≈950 行/分 ⇒ 全片约 30 min；**本轮未跑完**，见 §9） |

## 6. 前 1000 帧对齐

### 6.1 比对协议（为什么这样比）

1. **帧轴**：Rust 帧 `N`（`rust_screenshot` 的 `frames`/`joy` 行是 1-based）消费录像第 `N-1` 帧输入，
   故喂给 Rust 的 `joy N` = BizHawk 轨迹 `f=N-1` 的 `p1`（同理 `joy2`）。
2. **相位**：Rust `--state-out` 每行 = `tick(N)` 的 **spin 出口采样缝**（post-NMI / pre-work，
   `crates/game-core/src/frame/engine.rs:112-126`）；BizHawk 侧每行 = 该帧 `emu.frameadvance()`
   返回后的 RAM 面。二者在此类「帧初不变、帧内推进」的量上同相位，实测 $14 锁存序列 **967/967 全同**
   （§6.3）即该协议的直接验证。
3. **上电窗口排除**：BizHawk/quickerNES 上电 RAM 填 `0xFF`（`f=0..19` 四字段全 `0xFF`，其中
   `status` 在 f2 被复位代码清 0），Rust 侧 `f=1` 起为 `0x00`（boot 清 RAM）。这是**模拟器上电
   RAM 填充值差异**，不是游戏行为差异 ⇒ 比对窗口从 `f=20` 起算。
4. **缺行排除**：Rust 侧 `f∈{20,21,22,23,24,25,26, 482,483, 673,674,675, 946,947}`（14 帧）
   **没有状态行**——不是崩溃、不是跳帧，而是这些帧落在 **iteration span / title 续段**，
   asm 侧同期**无 `$C0D3` spin 出口**故无采样缝（`crates/game-core/src/frame/engine.rs:112-126`
   的 `!in_span && !title_continuation && phase∈{PostSpin,MainLoop}` 条件；同注释记录
   「emu f4-6/f311-313 缺行实录」）。判定证据：整程 stderr **无** `[rust] ⚠ 帧 N 引爆缺口`
   （若 tick panic，`crates/game-core/src/bin/rust_screenshot.rs:558-570` 会 `break` 整个循环，
   而轨迹一直写到 f1200 ⇒ 无 panic）。实测 stderr **无** `[rust] ⚠ 帧 N 引爆缺口` 行
   （`grep -c '引爆缺口' /tmp/rust-stderr.log` = 0）。
   附注：stderr 的 `[rust]   缺口记账 3020/6`（`crates/game-core/src/bin/rust_screenshot.rs:629`）
   = 「占位臂缺口事件数 3020 / 唯一地址 6」，属**非致命记账**（sound/占位臂 `_ => None`），
   与「帧无状态行」无关，不要混淆。

### 6.2 逐帧比对结果（f=20..1000）

| 指标 | 数字 |
|---|---|
| 窗口帧数 | 981（`f=20..1000`） |
| 可比帧（两侧都有行且跳过上电窗口） | **967** |
| 四字段**全同** | **957**（98.97%） |
| 有失配的帧 | **10** |
| `health_lo` 失配 | **0** |
| `health_hi` 失配 | 2（f28,f29） |
| `status` 失配 | 4（f28,f29,f952,f953） |
| `room` 失配 | 6（f27,f944,f945,f948,f949,f950） |
| `scrolldir` 失配 | 1（f940） |
| `$0014` 输入锁存失配 | **0 / 967** |

**首个分歧帧 = f27，字段 `room`：Rust `0xFF` / BizHawk `0x09`**（上电窗口后的首个差异，见 §6.4 表）。
**首个「游戏内」分歧帧 = f944，字段 `room`：Rust `0xFF` / BizHawk `0x09`**。

失配帧完整清单：

| f | 字段 | Rust | BizHawk | 性质 |
|---|---|---|---|---|
| 27 | room | `FF` | `09` | 上电窗口边缘：biz 在 f26-27 写 `$5A=09`，Rust 该区间无采样缝（f20-26 缺行）且 tick-close 采样也未观测到 09 |
| 28 | health_hi / status | `03` / `20` | `00` / `00` | Rust 进入游戏早 2 帧（biz 在 f30 才到 `03/20`）⇒ 属 boot 相位差，非数值差 |
| 29 | health_hi / status | `03` / `20` | `00` / `00` | 同上 |
| 940 | scrolldir | `02` | `03` | 门外过渡起始晚 1 帧（`$49` 写 3 的时机） |
| 944 | room | `FF` | `09` | 见 §6.4 |
| 945 | room | `FF` | `09` | 同上 |
| 948 | room | `F1` | `F4` | 过渡序列滞后 2 帧 |
| 949 | room | `F2` | `FF` | 同上 |
| 950 | room | `F4` | `FF` | 同上 |
| 952 | status | `01` | `00` | 过渡收尾滞后 1 帧 |
| 953 | status | `00` | `03` | 同上 |

（除上述 10 帧外，`f=30..939` 区间内 **905 个可比帧四字段全同**（区间 910 帧 − 5 个无采样缝帧
482,483,673,674,675）——即从进入游戏到门外过渡前，`health/status/room` 逐帧完全一致，
`$49` 亦一致，`$0014` 输入锁存全同。）

### 6.3 帧轴对齐的直接证据（不是拟合出来的）

Rust `zp[0x14]`（= `$0014` 锁存）vs BizHawk `joy1_latch`，在 967 个可比帧上**逐帧相同（967/967）**，
包括首个非零（两侧都在 f14=`0x10` Start）、第二次（f18=`0x10`）、首个方向键（f229=`0x02` Left）。
⇒ **Rust 帧 N ↔ BizHawk 帧 N**（同一帧号消费同一输入），不存在整体 ±1/±2 错位；§6.2 的 10 帧失配
是**局部的时序差**，不是对齐错。

### 6.4 room 过渡段（f=938..955）逐帧实证

`rust-seam` = `--state-out` 状态行（spin 出口相位）；`rust-close` = `capture` 的 `*.ram.hex`
（tick `f-1` 收口相位，逐帧采，用于给缺行帧（946/947）补观测面）。

| f | biz room/status/scrolldir | rust-seam | rust-close(tick f-1) room |
|---|---|---|---|
| 938 | `FF`/`01`/2 | `FF`/`01`/2 | `FF` |
| 939 | `FF`/`01`/2 | `FF`/`01`/2 | `FF` |
| 940 | `FF`/`01`/3 | `FF`/`01`/**2** | `FF` |
| 941 | `FF`/`01`/3 | `FF`/`01`/3 | `FF` |
| 942 | `FF`/`01`/3 | `FF`/`01`/3 | `FF` |
| 943 | `FF`/`01`/3 | `FF`/`01`/3 | `FF` |
| 944 | **`09`**/`01`/3 | `FF`/`01`/3 | `FF` |
| 945 | **`09`**/`01`/3 | `FF`/`01`/3 | **`F1`** |
| 946 | `F1`/`01`/3 | （无行） | `F1` |
| 947 | `F2`/`01`/3 | （无行） | `F1` |
| 948 | `F4`/`01`/3 | `F1`/`01`/3 | `F2` |
| 949 | `FF`/`01`/3 | `F2`/`01`/3 | `F4` |
| 950 | `FF`/`01`/3 | `F4`/`01`/3 | `FF` |
| 951 | `FF`/`01`/3 | `FF`/`01`/3 | `FF` |
| 952 | `FF`/**`00`**/3 | `FF`/`01`/3 | `FF` |
| 953 | `FF`/**`03`**/3 | `FF`/`00`/3 | `FF` |
| 954 | `FF`/`03`/3 | `FF`/`03`/3 | `FF` |

定谳（只陈述实测，不推断机制）：
- **`$5A=09` 这一中间值在 Rust 侧缺失**：biz 在 f944/f945 = `09`；Rust 在同一 tick 内 `FF→F1`
  （`rust-close` f945 = `F1`），**逐帧 tick-close 采样下从未出现 `09`**。
- **`F1→F2→F4→FF` 序列 Rust 侧晚 2 帧**（biz f946-949 vs rust f948-951）。过渡结束后两侧重新收敛
  （f954 起全同），`status` 的 `01→00→03` 相应晚 1 帧（f952/f953）。
- 全 0..1200 范围 `$5A` 取值集对比：biz = {`00`,`08`,`09`,`14`,`17`,`F0`,`F1`,`F2`,`F3`,`F4`,`FF`}；
  Rust = {`00`,`F0`,`F1`,`F2`,`F3`,`F4`,`FF`}——**`08/09/14/17`（房间号类值）在 Rust 侧未观测到**。
  说明：这些值可能写在 span/无缝帧内且被同帧覆写，故本报告只记「未观测到」，不断言「不写入」。

## 7. 【证伪登记】`tools/movie_import.py` 的 bk2 位序缺陷（本轮发现，未修改）

**现象**：bk2 的 `Input Log.txt` 每行 3 组 `|creg|p1|p2|`，其位序由同文件的 `LogKey` 行声明：
```
LogKey:#Reset|Power|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|P1 B|P1 A|#P2 Up|...
```
即 **P1 = U,D,L,R,Start,Select,B,A**（UDLRTSBA 序）。而
`tools/movie_import.py:128` `bits_from_text()` 按 **fm2 的 `RLDUTSBA`**（`tools/movie_import.py:48`
`BTN_ORDER_TEXT`）解码 `1<<i`，且 `parse_bk2`（`tools/movie_import.py:88-125`）**完全不读 `LogKey`**。

**实证（A 级）**：

| 证据 | 内容 |
|---|---|
| 录像原文 | f228 行 = `|..|..L.....|........|`，第 2 列（0-based）= `LogKey` 的 `P1 Left` |
| BizHawk 录像解析 | `movie.getinput(228)` → `P1 Left`；轨迹 `f228.p1 = 2`（Left=bit1） |
| ROM 实际消费 | 轨迹 `f229.joy1_latch = 2`（`$0014`=Left，晚一帧锁存） |
| 仓库 inputlog | `movies/eien86-6725M.inputlog` 第 229 行 = `04`（= Down） |

**错位映射**：`idx0 Up→0x01(Right)`、`idx1 Down→0x02(Left)`、`idx2 Left→0x04(Down)`、
`idx3 Right→0x08(Up)`；`Start/Select/B/A` 4 位位置巧合正确（故「Start 看起来正常」）。

**影响量化**（实测）：用既有 inputlog 喂 Rust 腿 1200 帧，再与 BizHawk 轨迹比：
- 输入面首分歧 **f229**：Rust `$14=0x04`（Down）vs BizHawk `0x02`（Left）；
- 状态面首分歧 **f467**（`status` Rust `00` vs biz `01`）；
- `f=20..1000` 失配帧数 **535**（对照：用 BizHawk 轨迹输入 = **10**）。

**处置**：本次**不修改** `tools/movie_import.py`，也**不修改** `movies/*.inputlog`
（取证纪律：不动既有证据链；且修工具会牵动既有对拍基线）。本轮对齐改用 **BizHawk 轨迹的 `p1/p2`
（= 录像真值）**作为「同一份逐帧输入」喂 Rust 腿，故 §6 的结论不受该缺陷影响。
**待办**：`movie_import.parse_bk2` 应改为按 `LogKey` 动态建列映射；修后需重跑依赖 bk2 inputlog 的
既有对拍基线（单独 commit）。

> 【2026-11-18 后续】该待办已由 commit `77302b66` 完成（`parse_bk2` 改为解析 `LogKey` 建列描述表，
> 缺 `LogKey` 即 fail-loud 拒绝；`bits_from_text` 加列序参数，fm2 仍 RLDUTSBA；新增合成位序锚
> `tools/tests/test_movie_import.py`）。本节保留当时「未修改」的历史主张不改；§6 的比对与结论
> 不受影响（本轮一直用 BizHawk 轨迹 `p1/p2` 作真值输入，未用该 inputlog）。
> 仍在的待办：`movies/eien86-6725M.inputlog` 是旧实现产物 ⇒ 需按修复后工具**重新生成**，
> 并重跑所有依赖它的对拍基线（属"重生成产物 + 基线"，与本节取证结论无关）。

> 【2026-11-18 二次后续：该待办已销账，并暴露本节一处行号笔误】
> 1. `movies/eien86-6725M.inputlog` 的 mtime 已是 `2026-11-18 09:38`，即已按修复后工具重新生成。
>    复核方式：`python3 tools/movie_import.py convert <bk2> -o /tmp/inputlog-full.txt` 后逐帧比，
>    **28408/28408 行 `diffs: 0`** ⇒ 跟踪产物 == 当前解析器输出（不是「大概修好了」，是逐帧恒等）。
> 2. **行号笔误更正**：上表「仓库 inputlog 第 229 行 = `04`」漏算了 9 行 `#` 元信息头
>    （`.inputlog` 结构 = 9 行 `#` + 每帧一行 ⇒ **帧 f 在第 `10+f` 行**）。正确锚点是
>    **第 238 行（f228）= `02`（Left）**；配套锚点 **第 23 行（f13）= `10`（Start）**。
>    当前产物在上列两行给 `02`/`10`，与 BizHawk `movie.getinput` 一致。
>    `第 229 行 = 04` 这一条是**修复前**产物的读数（当时确实为 `04`），保留作历史证据。
> 3. 修复后产物喂 Rust 腿再比（`movie_sched.py --frames 1200` → `rust_screenshot --state-out`
>    → `compare_bizhawk_rust.py --inputlog … --start 20 --end 1000`）：
>    `可比 967` / `四字段全同 957` / `失配 11` / **`$0014` 锁存失配 `0/967`** /
>    **`inputlog`↔BizHawk `p1` 失配 `0`**，失配段 = `f27..29 / f940 / f944..945 / f948..950 / f952..953`
>    ⇒ 与 §6.2、§10.4 完全一致；本节「影响量化」里那个 **535 帧**失配是**错误产物**的读数，
>    不得再当现状引用。
> 4. 因此本节的可执行结论改为：**位序缺陷已修 + 产物已重生成 + 输入面 0/967 失配**；
>    历史主张（缺陷存在、错位映射、535 帧）只作为「若再出现同类静默错位，会造成什么规模」的量化参照。

## 8. 复现命令

```bash
# 0) Rust 腿状态行（读 BizHawk 轨迹输入 → 临时调度表 → 逐帧状态）
cd /home/peng/workspace/metroid-decomp
python3 - <<'PY'
import json
rows=[json.loads(l) for l in open('analysis/traces/bizhawk-6725M.jsonl')]
out=["name bizhawk6725m","frames 1200"]
for r in rows:
    if r['f']>=1200: break
    f=r['f']+1
    if r['p1']>0: out.append("joy %d %02x"%(f,r['p1']))
    if r['p2']>0: out.append("joy2 %d %02x"%(f,r['p2']))
open('/tmp/bizhawk6725m.sched','w').write("\n".join(out)+"\n")
PY
cargo run -q -p game-core --bin rust_screenshot -- \
  --schedule /tmp/bizhawk6725m.sched --out /tmp/rustshots --state-out /tmp/rust-states.jsonl

# 1) BizHawk 侧逐帧轨迹（无头；Xvfb + 关音频；输出为绝对路径）
cd /tmp/bh
ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy timeout 240 xvfb-run -a ./EmuHawkMono.sh \
  --chromeless --lua /home/peng/workspace/metroid-decomp/tools/movie_ref/bizhawk_ref.lua \
  --movie "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" \
  "/home/peng/workspace/metroid-decomp/rom/Metroid (USA).nes"
# 全片（约 30 min，本轮未跑完）：加 BH_FRAMES=28408

# 2) 逐帧比对（输出 §6.2 的全部数字）
python3 - <<'PY'
import json
B={}
for l in open('analysis/traces/bizhawk-6725M.jsonl'):
    r=json.loads(l); B[r['f']]=r
R={}
for l in open('/tmp/rust-states.jsonl'):
    r=json.loads(l); zp=r['w']['zp']; i=r['w']['iram']
    R[r['f']]=dict(hl=int(i[12:14],16),hh=int(i[14:16],16),st=int(i[1024:1026],16),
                   rm=int(zp[180:182],16),j1=int(zp[0x14*2:0x14*2+2],16),
                   sd=int(zp[0x49*2:0x49*2+2],16))
comp=[f for f in range(20,1001) if f in R]
eq=lambda f:(R[f]['hl'],R[f]['hh'],R[f]['st'],R[f]['rm'])==(B[f]['health_lo'],B[f]['health_hi'],B[f]['status'],B[f]['room'])
bad=[f for f in comp if not eq(f)]
print("窗口20..1000:",981,"可比:",len(comp),"缺行:",981-len(comp),
      "全同:",len(comp)-len(bad),"失配:",len(bad),bad)
print("$14 失配:",[f for f in comp if R[f]['j1']!=B[f]['joy1_latch']])
for f in bad:
    d=[(k,R[f][k],{'hl':B[f]['health_lo'],'hh':B[f]['health_hi'],
                   'st':B[f]['status'],'rm':B[f]['room']}[k])
       for k in ('hl','hh','st','rm')
       if R[f][k]!={'hl':B[f]['health_lo'],'hh':B[f]['health_hi'],
                    'st':B[f]['status'],'rm':B[f]['room']}[k]]
    print("f%-4d %s" % (f, d))
PY
```

`--state-out` 行解码：`zp`=`$0000-$00FF`（每字节 2 hex，偏移 `addr*2`）、
`iram`=`$0100-$07FF`（偏移 `(addr-0x100)*2`）——契约见
`crates/game-core/src/memmap/observe/frame_face.rs:9-11,32-34`。

## 9. 未做 / 限制（不得当结论用）

1. **全片轨迹未导出**：只导出 `f=0..1200`（1201 行）。`BH_FRAMES=28408` 实测可跑（跑到 6669 行时估 ≈950 行/分，粗测），
   全片约 30 min，本轮未完成 ⇒ 之后的帧（f>1200）无取证。
2. **未定位 quickerNES fork 的具体版本**（录像未记录 commit）⇒ 只声称「同源同名核心」，不声称
   「与录制时逐指令同版本」。
3. **`$5A=09` 缺失与门过渡 ±1~2 帧时序差的机制未考证**：本节只给观测，不给机制结论；
   需要时按 `crates/game-core/src/world/room_draw.rs:720,826,851`（`$5A` 写入点）+
   `crates/game-core/src/world/room_load.rs:57,202` 逐点插桩再定谳。
4. **未做像素/PPU 面比对**（`capture` 只用于补房间号观测面）；本轮只比 RAM 四字段 + `$49` + `$14`。
5. **未改任何游戏逻辑/门禁/阈值**；`rom/` 未触碰；临时用例只在 `/tmp`（`cases/` 未新增文件）。

## 10. 全片对拍（2026-11-18 补做）：RAM 面 f=20..28407 + 像素面 f=20..1200

本节是 §6 的**全片续做**，不修改 §6 既有结论；§9 的 4 条「未做」在本节被部分销账（见 §10.8）。

对拍两腿：**BizHawk 2.10 + quickerNES**（录像作者核心）↔ **Rust 重制**（`rust_screenshot`）。
只做取证与比对，**未改任何游戏逻辑**。

### 10.0 先修工具：BH_FRAMES 闭区间 off-by-one（这是「挂死」的真因）

`tools/movie_ref/bizhawk_ref.lua` 旧实现是 `for frame = 0, TOTAL_FRAMES do`（**闭区间**），
而 bk2 的帧数真值是 **28408 帧 ⇒ f=0..28407**（`Input Log.txt` 里 28408 行 `|…`）。
于是 §5/§8 记的「全片导出用 `BH_FRAMES=28408`」会去迭代**越界帧 28408**：

| 阶段 | 命令 | 实测 |
|---|---|---|
| 修正前 | `BH_FRAMES=28408` | 跑到 f28401 处**僵死**（STAT `Sl`、CPU 53.9%、RSS 827 MB、输出文件 mtime 不再增长），被 `timeout` 杀；产物 **28401 行**（末 7 行留在 Lua `io.write` 缓冲里丢失——脚本只每 100 帧 `flush()`） |
| 修正前 | `BH_FRAMES=28407` | **28408 行 f=0..28407**，正常跑完（证明「越界帧」是唯一变量） |
| 修正后 | `BH_FRAMES=28408` | **28408 行 f=0..28407**，7 min 53 s 跑完，**不挂**；与本机 `BH_FRAMES=28407` 旧产物 **逐字节相同** |

修正 = 半开区间 `for frame = 0, TOTAL_FRAMES - 1 do`（`BH_FRAMES=N` ⇒ 导出 `f=0..N-1` 共 N 行），
并把 `tools/movie_ref/run_bizhawk_trace.sh` 的判成功口径从 `行数 == frames+1` 同步改为 `行数 == frames`。
回归验证（1200 帧窗）：修正后 `BH_FRAMES=1200` 产 1200 行，与修正前 1201 行产物的**前 1200 行逐字节相同**
（`cmp` 通过）⇒ 只去掉越界那一行，历史数字不受影响。

> 机制说明：Bash/Lua 侧没有别的证据能定位那次僵死的报错文本（`--chromeless` 下 Lua 报错会弹模态窗，
> 无人点击 ⇒ 进程永远等待；`/tmp/bh/*.log` 里只有启动期 X11 BadMatch 栈）。所以本条结论用的是
> **对照实验**（28408 挂 / 28407 不挂 / 修正后 28408 不挂且产物等价），不是报错文本。

### 10.1 全片轨迹（BizHawk 腿）

| 项 | 值 |
|---|---|
| 文件 | `analysis/traces/bizhawk-6725M-full.jsonl`（`analysis/` 不入库） |
| 行数 / 帧范围 | **28408 行，f=0..28407** |
| 耗时 | **7 min 53 s**（birth 10:15:21 → mtime 10:23:14，单进程、单段导出、无合并） |
| 末行 | `{"f":28407,"health_lo":72,"health_hi":0,"status":1,"room":255,"scrolldir":0,"p1":8,...}` |
| 退出码 | 137（BizHawk shutdown 后自杀 ⇒ 预期；判定只看行数，见 §3） |

### 10.2 Rust 全片重放

`movies/eien86-6725M.inputlog`（§7 修好位序的那份）→ `tools/movie_ref/movie_sched.py` → `/tmp/m6725-full.sched`
→ `rust_screenshot --schedule /tmp/m6725-full.sched --out /tmp/rustshots-full --state-out /tmp/rust-states-full.jsonl`：

| 项 | 值 |
|---|---|
| 耗时 | **18.82 s**（`/usr/bin/time`，峰值 RSS 1 674 876 KB） |
| 状态行 | **28308 行，f=1..28408** |
| 缺行 | **100 帧**：iteration span / title 续段不落状态行（`crates/game-core/src/frame/engine.rs:112-126`），见 §10.3 的 30 段清单 |
| 采集 | 136 张 PNG + `.ppu.hex` + `.ram.hex`（f=20..1200 步长 10 + f=938..956 逐帧） |

### 10.3 RAM 面对拍：结果

判定面 5 项（地址锚见 §4 表；`status` 语义见 `crates/game-core/src/samus/state.rs:48-110` 的 `sa_*` 十态）：
`health_lo=$0106`、`health_hi=$0107`、`status=$0300`、`room=$005A`、`scrolldir=$0049`；
外加 `joy1_latch=$0014` 作**帧轴证据**（不是判定项）。

| 项 | 值 |
|---|---|
| 窗口 | **f=20..28407**（28388 帧） |
| 可比帧 | **28295** |
| Rust 无状态行（非失配） | **93** 帧 / 30 段 |
| BizHawk 无行 | 0 |
| 五字段全同 | **1809** |
| 四字段全同（不含 `scrolldir`） | **2280** |
| 失配帧 | **26486** |
| 逐字段失配 | `health_lo=23055`、`health_hi=14248`、`status=21071`、`room=1546`、`scrolldir=15554` |
| 失配区间（严格连续） | **222 段** |
| `$0014` 锁存失配 | **223/28295**（首 f1133） |
| `inputlog`↔BizHawk `p1` 失配 | **0**（两腿喂的逐帧输入同源，已独立校验） |

**这个「26486 失配」必须按两段读，不能当 26486 个独立缺陷**：

| 段 | 可比帧 | 失配 | 全同 | 逐字段失配 |
|---|---:|---:|---:|---|
| **f=20..1131（分岔前）** | 1098 | **73** | **1025** | `status=66`、`room=6`、`health_hi=2`、`scrolldir=1`、`health_lo=0` |
| **f=1132..28407（分岔后）** | 27197 | 26413 | 784 | 级联结果（两腿已不在同一场景） |

分岔前的 73 帧 = §6.2 的 11 帧（f27..29、f940、f944..945、f948..950、f952..953）+ §10.5 的 f1031..1081（51 帧）
+ 其后 11 个单帧 `status` 抖动（f1093/1097/1101/1105/1108/1112/1116/1120/1123/1127/1131）。

> 【2026-11-18 复核更正（r9，独立 verifier 实测）】本节三处数字按复核结果改正：
> ①分岔后失配 26408 → **26413**（27197−784；且 26486−73=26413 自洽）；
> ②分段桶 `f15020..16019` 215/0/215 → **996/215/781**（原表列错位），`f28020..28400` 381/0/381 → **f28020..28407 388/0/388**（原口径混用了 `--end 28400` 那次 run）；
> ③「最后一次全同出现在 f24457 之后即无」→ 真值是 `equal_runs` 末段止于 **f15527**，`f15528..28407` 共 **12880 帧零全同**。

**全同区间**（198 段）前 6 段：`f30..481`、`f484..672`、`f676..939`、`f941..943`、`f951`、`f954..1030`
⇒ **f=30..1030 除 §6.2 那 11 个已知点外逐帧全同**（含 `$0014`），与 §6.2 的分部结论一致。

**失配区间表（前 10 段；完整 222 段 + 逐帧明细落在 `analysis/traces/ram-compare-full.md`）**：

| # | 起 | 止 | 帧数 | 字段 | 右侧样本（Rust / BizHawk） |
|---:|---:|---:|---:|---|---|
| 1 | 27 | 29 | 3 | `health_hi`,`room`,`status` | f27 `room` `FF`/`09`；f28..29 `health_hi` `03`/`00`、`status` `14`/`00` |
| 2 | 940 | 940 | 1 | `scrolldir` | `02`/`03` |
| 3 | 944 | 945 | 2 | `room` | `FF`/`09` |
| 4 | 948 | 950 | 3 | `room` | `F1`/`F4`、`F2`/`FF`、`F4`/`FF` |
| 5 | 952 | 953 | 2 | `status` | `01`/`00`、`00`/`03` |
| 6 | 1031 | 1081 | 51 | `status` | f1031 `03`/`04`，f1032 起 `03`/`01` |
| 7 | 1093 | 1093 | 1 | `status` | `00`/`01` |
| 8 | 1097 | 1097 | 1 | `status` | `00`/`01` |
| 9 | 1101 | 1101 | 1 | `status` | `00`/`01` |
| 10 | 1105 | 1105 | 1 | `status` | `04`/`01` |

末 3 段（分岔后一路到片尾）：`[220] f24027..24197（171 帧）`、`[221] f24200..24455（256 帧）`、
`[222] f24458..28407（3950 帧）`。
**全同区间的最后一段止于 f15527**（末 3 段 `[15518..15520]`/`[15522..15523]`/`[15525..15527]`）
⇒ **f15528..28407 共 12880 帧零全同**（不是「f24457 之后无全同」——那个说法把零全同窗口少算了约 9000 帧）。

Rust 无状态行的 30 段（全部列出，**不计失配**）：
`20..26`、`482..483`、`673..675`、`946..947`、`1515..1516`、`2466..2467`、`3277..3278`、`3785..3786`、
`4933..4935`、`5094..5098`、`5538..5544`、`6122..6123`、`8588..8589`、`8821..8822`、`9395..9396`、
`10470..10471`、`10658..10660`、`10819..10823`、`13864..13870`、`14327..14328`、`15584..15585`、
`15800..15801`、`16499..16501`、`16660..16664`、`21458..21464`、`21919..21920`、`22201..22202`、
`24025..24026`、`24198..24199`、`24456..24457`。

分段（每 1000 帧，可比/全同/失配）——可见中段有若干「重新收敛岛」：

| 桶 | 可比 | 全同 | 失配 | | 桶 | 可比 | 全同 | 失配 |
|---|---:|---:|---:|---|---|---:|---:|---:|
| f20..1019 | 986 | **975** | 11 | | f10020..11019 | 990 | 9 | 981 |
| f1020..2019 | 998 | 365 | 633 | | f11020..12019 | 1000 | 0 | 1000 |
| f2020..3019 | 998 | 0 | 998 | | f15020..16019 | 996 | 215 | 781 |
| f8020..9019 | 996 | **135** | 861 | | f28020..28407 | 388 | 0 | 388 |
| f9020..10019 | 998 | **110** | 888 | | 其余中段桶 | ≈1000 | 0 | ≈1000 |

### 10.4 首个失配：f27（`room`）

```
f27  rust: health_lo=00 health_hi=00 status=00 room=FF scrolldir=02
f27  biz : health_lo=00 health_hi=00 status=00 room=09 scrolldir=02
```
即**只有 `room` 一项不同**，与 §6.2 一致（上电后 BizHawk 侧 `$5A` 已是 09，Rust 侧仍是 `FF`）。
性质与 §6.4 的 `$5A=09` 缺失同源（机制仍未考证，见 §9.3 / §10.8）。

> 【2026-11-18 更正】§6.2 汇总行写「有失配的帧 = **10**」，但其明细表列了 **11** 行；
> 本节实测窗口 20..1000 同样是 **11**（f27,f28,f29,f940,f944,f945,f948,f949,f950,f952,f953）。
> 该表其余数字（窗口 981、可比 967、四字段全同 957、逐字段 `0/2/4/6/1`、`$0014` 0/967）本轮**逐项复现**。
> ⇒ §6.2 的「10」是算术笔误，按 11 读。

### 10.5 首次实质性分歧：f1031（变形球退出，晚 50 帧）

f1010..f1030 两侧全同且 `status` 都是 `03`（`sa_Roll`，变形球）。f1030 作者的 `p1=09`（`Up|Right`）是退球手势：

| f | Rust `status` | BizHawk `status` | 说明 |
|---:|---|---|---|
| 1031 | `03` Roll | `04` PointUp | biz 已站起（PntUp 是退球那一帧的姿态） |
| 1032..1080 | `03` Roll | `01` Run | biz 在跑，rust 仍卡在球里（**49 帧**） |
| 1081 | `00` Stand | `01` Run | rust 这一帧才站起 |
| 1082..1092 | `01` Run | `01` Run | **重新收敛** |

f1031..1080 期间 `p1` 恒为 `01`（无 `Up`），Rust 却在 f1081 自己站起来了 ⇒ 退球触发条件/时序与 ROM 不同
（**假设级，未定谳**；需要按位球状态机插桩才能给出机制结论）。

其后 f1093..1131 的 11 个单帧 `status` 抖动（`00` Stand 每约 4 帧插一帧、`04` PointUp 偶发）也落在此段，
biz 全程 `01`。**f1132 是分水岭**：biz 的 `$5A` 依次写 `14→F1→F1→F2→F4→FF`（门过渡真实发生），
Rust 的 `$5A` 全程 `FF`（**没进门**）⇒ 此后两腿处于不同场景，$0014 锁存失配也从 f1133 开始（累计 223 帧）。

后段抽样（说明「分岔后」的性质，不是独立缺陷）：
`f2000` rust `st=04`/biz `00`；`f3000` rust `hp=9000 st=00`/biz `hp=2002 st=09`（Elevator）；
`f5000` rust `hp=0000 st=08`（Dead2）/biz `hp=2002 st=03`；f12000/f18000/f20000 同样是 rust 死亡态而 biz 存活。
`room=$5A` 两侧全片基本都停在 `FF`（只有门过渡窗口会写中间值）。

### 10.6 像素面前段 f=20..1200

**用哪条腿：Mesen2 腿**（`tools/mesen2/Mesen`，经 `tools/scenario_run.py` 驱动），**不是** BizHawk 腿。
原因实测：BizHawk `client.screenshot(path)` 可用、也能采到 sprite 与 HUD，但其输出是 256×224（Rust 是 256×240，
最佳裁切 y=8..231）且**调色板差 ±1/通道**（biz `#007C8E` vs rust `#007C8D`、`#9390FE` vs `#9290FF`），
零容差比对不可能成立；Mesen2 调色板与 Rust 一致（既有 L4 门禁在 `shots/` × `oracle/screens/` 上本来就 0 px）。

覆盖：**136 个采集点**（f=20..1200 步长 10 共 119 点 + f=938..956 逐帧 19 点，其中 938/939/940/941/942/943… 与步长点去重后净增 17）。

判定：`tools/movie_ref/compare_pixels.py` → `tools/diff_visual.diff_frames(name, golden, fresh, wl=[], gaps=[])`
（**与 L4 门禁同一判定实现**，且**空白名单** ⇒ 比门禁更严）。

| 结果 | 值 |
|---|---|
| 0 px | **83/136** |
| 有分歧 | **53/136** |

逐采集点（等值点压缩成区间；完整 136 行逐点表落 `analysis/traces/pixel-compare.md`）：

| 采集点 | 点数 | 每点分歧 px |
|---|---:|---:|
| f20 | 1 | 350 |
| **f30..f660** | 64 | **0** |
| f670 | 1 | 9076 |
| f680 | 1 | 9180 |
| f690..f920 | 24 | 9178 |
| f930 | 1 | 9275 |
| f938..f941 | 4 | 9270 |
| f942 | 1 | 9264 |
| f943 | 1 | 9078 |
| f944 | 1 | 9130 |
| f945 | 1 | 9066 |
| f946 | 1 | 8842 |
| f947..f948 | 2 | 8973 |
| f949 | 1 | 8948 |
| f950 | 1 | 8823 |
| f951 | 1 | 8808 |
| f952 | 1 | 9009 |
| f953..f980 | 7 | 8808 |
| **f990..f1100** | 12 | **0** |
| f1110 | 1 | 80 |
| f1120 | 1 | 0 |
| f1130 | 1 | 80 |
| f1140..f1150 | 2 | 0 |
| f1160 | 1 | 80 |
| f1170..f1200 | 4 | 0 |

**分歧性质（`tools/movie_ref/pixel_shift_probe.py` 整数平移探针）**：

1. **f670..f980（49 点，≈8808～9275 px）不是内容差，是 BG 的 1 px 水平相位差。**
   对该段所有点做 (dx,dy)∈[-3,3]² 的整屏搜索：最优一致解是 **(dx=1, dy=0)**，重叠区
   61200 像素中残差仅 **261**（f670=312、f680=325、f960..f980=157）；dx=0 时残差 ≈9178。
   `f690/f800/f900` 三点数值完全相同 ⇒ 这段屏幕是静止的（同一画面重复采样）。
   dx=1 后残余 261 px 的 bbox 是 x=24..122 / y=34..145（分散小对象区）⇒ **BG 整体偏 1 px、sprite/OAM 完全一致**。
   探针在 f20 上任何平移都消不掉（350 px 不变）⇒ 那是真内容差（见第 2 条），不是相位差。

2. **f20（上电期）350 px**：bbox x=96..158 / y=96..126，全部是 emu=`000000FF`（黑）↔ rust=`C0DFFFFF`。
   即 f20 时 Rust 已经画出画面而 emu 还是黑的——与 §6.1 的 f0..19 上电填充差同族，**不是渲染错误**。

3. **f1110 / f1130 / f1160 各 80 px**：bbox 都是 **x=135..147 / y=193..208**（Samus sprite 所在），
   颜色在 `EA9E22`/`B53120`/黑之间互换；f1130、f1160 与 f1110 极性相反 ⇒ **sprite 动画相位差**（非平移、非调色板）。
   这三点正好落在 §10.5 的 `status` 抖动段（f1093..1131）与 f1160 —— 与 RAM 面首次实质性分歧同源。

> 像素面的帧相位约定与既有 L4 矩阵一致：Rust `capture N` ↔ emu 帧 N 截图（`rust_screenshot.rs:484-520` 的
> 「capture 帧 N = tick(N-1) 双相位」）。f990..f1100 这 12 个静止点 0 px、`f30..f660` 64 点 0 px，
> 反证**帧相位不错位**（否则静止窗也会整屏差）。

### 10.7 复现命令（本机实测，路径全部参数化）

```bash
# 0) 前提：BizHawk 2.10 在 $BIZHAWK_DIR（默认 /tmp/bh）；ROM=rom/Metroid (USA).nes；录像在 /tmp
# 1) 全片轨迹（28408 行 f=0..28407，实测 7 min 53 s）
TIMEOUT=1800 tools/movie_ref/run_bizhawk_trace.sh \
  "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" 28408 \
  analysis/traces/bizhawk-6725M-full.jsonl

# 2) Rust 全片重放（实测 18.82 s）
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog --frames 28408 \
  --capture-every 10 --capture-from 20 --capture-to 1200 \
  $(for f in $(seq 938 956); do printf -- '--capture %s:d%s ' $f $f; done) \
  --out /tmp/m6725-full.sched
cargo run -q -p game-core --bin rust_screenshot -- \
  --schedule /tmp/m6725-full.sched --out /tmp/rustshots-full --state-out /tmp/rust-states-full.jsonl

# 3) RAM 面全片对拍（打印摘要 + 完整区间表落盘）
python3 tools/movie_ref/compare_bizhawk_rust.py \
  --bizhawk analysis/traces/bizhawk-6725M-full.jsonl --rust /tmp/rust-states-full.jsonl \
  --inputlog movies/eien86-6725M.inputlog --end 28407 \
  --json-out analysis/traces/ram-compare-full.json \
  --md-out analysis/traces/ram-compare-full.md

# 4) 像素面：emu 腿（Mesen2，场景由仓库工具生成 ⇒ 可复现，实测 31.7 s）
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog --frames 1200 \
  --capture-every 10 --capture-from 20 --capture-to 1200 \
  $(for f in $(seq 938 956); do printf -- '--capture %s:d%s ' $f $f; done) \
  --format mesen2 --out /tmp/eien86-6725M.txt
python3 tools/scenario_run.py run /tmp/eien86-6725M.txt --timeout 600      # 截图落 shots/

# 5) 像素逐点 diff（空白名单）+ 平移探针
python3 tools/movie_ref/compare_pixels.py --oracle-dir shots --rust-dir /tmp/rustshots-full \
  --name-re 'eien86-6725M_' --json-out analysis/traces/pixel-compare.json \
  --md-out analysis/traces/pixel-compare.md
python3 tools/movie_ref/pixel_shift_probe.py --oracle-dir shots --rust-dir /tmp/rustshots-full \
  --name-re 'eien86-6725M_' --only-diff --json-out analysis/traces/pixel-shift-probe.json
```

确定性证据：Mesen2 腿用仓库工具重生成场景后重跑，`shots/eien86-6725M_*.png` 全部文件的 md5 汇总前后同为
`cce608f64a2cb8de5eedf2485e72198b`；BizHawk 腿修正前后产物逐字节相同（§10.0）。

### 10.8 本节结论与不作的承诺

**已证实（A 级，有命令与数字）**：
1. 全片规格：BizHawk **28408 行 f=0..28407**（7 min 53 s），Rust **28308 行 f=1..28408**（18.82 s）。
2. RAM 面窗口 f=20..28407：可比 28295，全同 1809（2280 四字段），失配 26486，222 段。
3. **f=30..1030 除 11 个已知点外逐帧全同**（`$0014` 亦同）；首个失配 f27 仅 `room`（`FF`/`09`）。
4. 分岔前（f<1132）只有 **73 帧**失配，其中 `status` 独占 66 帧；**f1031 变形球退出晚 50 帧**是首次实质性分歧。
5. 像素面 f=20..1200：83/136 点 0 px；f670..f980 是 **BG 1 px 水平相位差**（dx=1 残差 261/61200，sprite 全同）；
   f1110/1130/1160 是 **sprite 动画相位差 80 px**；f20 350 px 是上电期差。
6. 工具修正：BH_FRAMES 半开区间（修正前后数字见 §10.0），runner 判定口径同步。

> 【2026-11-18 更正】§5 的「全片导出 … 粗测 ≈950 行/分 ⇒ 全片约 30 min」偏保守：实测
> **28408 行 7 min 53 s ≈ 3600 行/分**（同一机器两次一致，见 §10.0/§10.1）。

**假设级（未定谳，不得当结论用）**：
- f1031 退球触发条件差异的机制（球状态机？头顶空间判定？）——需要按位球状态机插桩。
- §9.3 的 `$5A=09` 缺失机制（本次只是又确认了它存在：f27、f944..945、f948..950）。
- f670..f980 的 BG 1 px 水平相位差**来源**（滚动寄存器写入时机？`$49` 优先级？）——本次只给出量，
  未做 PPU 逐寄存器取证；`$49`（`scrolldir`）在该段两侧同值（f940 除外），故不是 `$49` 本身不同。

**不作的承诺**：§6.2 之后的全片数字不代表「Rust 到此为止都正确」；f1132 之后两腿已不同场景，
该段的失配计数只说明「分岔已发生」，不能逐字段归因。像素面只覆盖 f=20..1200（10 帧步长 + 门过渡带逐帧），
f>1200 的像素面**未做**。

## 11. 跨模拟器互证 quickerNES ↔ Mesen2（同输入 f=20..1000，2026-11-18 补做）

**目的**：既有全部分析基线（`data/status/baselines/`、L3/L4 结论、`§6` 的 Rust 对拍）都是
**Mesen2 语境**下取得的；本次取证换成了 quickerNES。若两核在同输入下不一致，则旧基线不可迁移到
新语境，§6 的「失配 10 帧」也要重新解释。本节给出可复现的判定。

### 11.1 两侧轨迹与工具

| 侧 | 产出 | 生成方式 |
|---|---|---|
| quickerNES（BizHawk） | `analysis/traces/bizhawk-6725M.jsonl` | `tools/movie_ref/bizhawk_ref.lua`，见 §4/§8 |
| Mesen2 | `/tmp/tmpaxis2.jsonl`（`w.{Health,SamusStatus,RoomNumber,Joy1Latch}`） | 临时用例 `/tmp/tmpaxis2.txt`：`frames 1000` + `snapshot none` + `inputlog movies/eien86-6725M.inputlog` + 4 个 `watch`；`python3 tools/scenario_run.py run /tmp/tmpaxis2.txt` |

比对工具（本次新增，可复跑）：`tools/movie_ref/compare_cores.py`。它只做**字段映射 + 统计**，
不裁定谁对；帧轴差异必须用 `--shift` 显式声明（语义 `biz f = mesen f - shift`），
`--latch-align` 用 `$0014` 锁存序列反解 shift。

### 11.2 帧轴：shift=0 是游戏内正解，boot 段有 1~2 帧原生相位差

`--latch-align` 在 f20..1000 上给出 **shift=0**（`$0014` 一致率 **0.9786**）。但 boot 段两核并不同步：

| 事件 | Mesen2 f | quickerNES f | 差 |
|---|---|---|---|
| `$5A→09`（boot） | 24 | 26 | −2 |
| `$0300→$20` | 28 | 30 | −2 |
| `$0300→21` | 166 | 166 | 0 |
| `$0300→22` | 266 | 266 | 0 |

⇒ boot 相位差在 ~f40 前自愈，**f≥40 起同帧号可比**。这也解释了 §6.1 为何把比对窗口起点定在 f20
（再往前是上电填充差异，不是游戏行为差异）。

> 【2026-11-18 更正】本节结论建立在一个**行内不一致的旧轨迹文件**上：当时 `bizhawk_ref.lua`
> 先读 RAM 再 `emu.frameadvance()`，行 f 记「frame f 开工态」而行内 `p1` 是 frame f 输入。
> 改写为「先 advance 再读」（行 f = frame f 收工态）后实测 `post[f] == pre[f-1]` 28 388/28 388
> 全等 —— 相位改写是**重新编号不是物理位移**，故本节「shift=0」的实测口径作废；按新文件的
> 正确映射，状态面 `biz_to_mesen = +1`（锁存面 +2，恒偏一行），见
> `data/status/leg_offsets.json` 与 `docs/spec/oracle-verification-chain.md` R6。
> 上表 boot 段的 −2 仍是两侧原生相位差（这一条没被推翻）。

### 11.3 结果（f=40..1000，961 个可比帧）

```
$ python3 tools/movie_ref/compare_cores.py \
    --bizhawk analysis/traces/bizhawk-6725M.jsonl --mesen /tmp/tmpaxis2.jsonl \
    --start 40 --end 1000 --shift 0 --json-out /tmp/cores.json
窗口 f=40..1000，shift=0，可比 961 帧
逐字段失配：health_lo=0  health_hi=0  status=2  room=13  scrolldir=0
含失配帧 15；分组 {"attrib-672": 5, "attrib-944": 10}
```

**关键结构**：15 个失配帧**全部**落在两处 `$005A` attrib 链窗口内
（f672..679 共 5 帧、f944..953 共 10 帧）；**窗口之外 0 失配**。

> 【2026-11-18 更正】上表的 961 可比帧/15 失配帧是**旧文件 + shift=0** 口径的读数。按新文件
> 与状态面偏移重算（f20..1199、可比 1180、进分母 1147）得 **43 段**失配：phase 6 / transient 31 /
> value 6 / undecidable 0，其中**唯一干净持续值差是 `biz f1031..1081 status`**（= mesen 轴
> 1031..1081）；「窗口之外 0 失配」不再成立（新口径下窗口外至少有 f1030..1080 这一段）。
> 逐段判定见 `docs/findings/movie-6725m-residual-episodes.md`。

| 字段 | 失配帧 | 位置 | 性质 |
|---|---|---|---|
| `health_lo` / `health_hi` | **0** | — | 两核完全一致 |
| `scrolldir($49)` | **0** | — | 两核完全一致 |
| `status($0300)` | 2（f952、f953） | attrib-944 窗口尾 | 门过渡 `01→00→03` 慢 1 帧 |
| `room($5A)` | 13 | attrib-672 / attrib-944 | 链成员采样不同（下） |

`$5A` 链成员的**核间差异形状**（同一段过渡，两核采到的中间值不同）：

| 窗口 | Mesen2 采到的链 | quickerNES 采到的链 |
|---|---|---|
| f672.. | `8 → F0 → F1 → F3 → F4 → FF` | `8 → F0 → F2 → F3 → FF` |
| f944.. | `9 → F0 → F1 → F3 → F4 → FF` | `9 → F1 → F2 → F4 → FF` |

即两核**端点一致**（`FF`/`8`/`9`/`F0` 起点），但 `F1..F4` 中间值落在哪一帧不同 —— 因为
`EA19` 的 `inc` 推进速率与帧采样相位在两核不同（§3 已记：boot 段一帧可连推 4 格，游戏内一帧一格）。
⇒ `$5A` 的**中间值不可作跨核帧级证据**，只有「链是否出现 / 端点值」是机制级可比的。

### 11.4 对既有结论的三点影响

1. **Mesen2 基线可迁移**（限这五个字段）：`health_lo/health_hi/scrolldir` 零失配，
   `room` 在**非过渡期**零失配 ⇒ 旧基线与本次 quickerNES 轨迹不冲突，§6 的结论不需要因换核而重做。

   > 【2026-11-18 更正】「零失配」只在旧口径下成立；新口径（43 段）里窗口外的干净值差是
   > `status` 而非 `room`，`room` 的失配全部带 `during_load=true`。迁移结论不变，但依据要写成
   > 「`room` 的跨核差异全部落在加载窗内」（A 级，`leg_episodes.json`）。
2. **`$005A` 过渡期必须单列**：两核在过渡窗口内必然不等（采样相位），任何跨核比对都必须把
   attrib 链窗口排除或改为「端点比对」，否则会把相位差误判成逻辑差 —— 这正是 §4.1 的教训。
3. **强化 §5 裁定**：两核都在帧边界上出现过 `$5A=09`（Mesen f946/947、quickerNES f944/945），
   而 Rust 从未在 tick 收口相位出现过 `09` ⇒ 「`09` 未落帧边界」这条**不是**单核观测假象，
   是 Rust 侧真实差异形态（但**仍未证**是否被同 tick 覆写，见 §5.2 反证条件 1）。

   > 【2026-11-18 更正】「是否为单核假象」这一问已有定论且结论更强：Rust 在 **tick 945 内**
   > 完成 `FF→09→F0→F1`（`capture(945)=FF`、`capture(946)=F1`），`09` 的 2 帧驻留未建模，
   > 属**已证**差异（不是「形态存疑」）。见 `docs/findings/room-5a-transition-verdict.md` §5/§8。

### 11.5 边界与未做

- 只覆盖 f=20..1000、单个录像、单个房间过渡簇；`f>1000` 未做跨核互证。
- `shift` 由 `$0014` 反解，而 `$0014` 本身在 boot 段有 1~2 帧相位差 ⇒ f<40 的跨核结论**不可用**。
- 未做像素/音频面跨核比对（本次只做 RAM 面）。
- 两核的 CPU/PPU 时序差异未考证（属「为什么相位差 1~2 帧」，不在本次范围）。

### 11.6 复现命令

```bash
# 1) Mesen2 侧（临时用例，跑完即删，不入 cases/）
cat > /tmp/tmpaxis2.txt <<'CASE'
name tmpaxis2
frames 1000
snapshot none
inputlog movies/eien86-6725M.inputlog
watch 0300 SamusStatus
watch 005A RoomNumber
watch 0106 Health
watch 0014 Joy1Latch
endFrame 1000
CASE
timeout 900 python3 tools/scenario_run.py run /tmp/tmpaxis2.txt
cp analysis/traces/tmpaxis2.frames.jsonl /tmp/tmpaxis2.jsonl
rm -f analysis/traces/tmpaxis2*

# 2) 比对
python3 tools/movie_ref/compare_cores.py \
  --bizhawk analysis/traces/bizhawk-6725M.jsonl --mesen /tmp/tmpaxis2.jsonl \
  --start 40 --end 1000 --shift 0 --json-out /tmp/cores.json

# 3) 帧轴标定（若怀疑 shift；参数顺序与上一步不同，便于 grep 区分）
python3 tools/movie_ref/compare_cores.py \
  --mesen /tmp/tmpaxis2.jsonl --bizhawk analysis/traces/bizhawk-6725M.jsonl \
  --start 20 --end 1000 --latch-align
```
