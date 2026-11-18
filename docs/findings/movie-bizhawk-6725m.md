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
