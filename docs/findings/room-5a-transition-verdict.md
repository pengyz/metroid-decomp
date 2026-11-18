# 裁定书：`$005A`（RoomNumber）过渡序列 —— ROM 写序列 vs Rust 写序列（BizHawk 6725M 对齐副产品）

- 日期：2026-11-18
- 触发：`docs/superpowers/plans/2026-11-18-bizhawk-movie-forensics-next.md` Task 3/4
- 上游证据：`docs/findings/movie-bizhawk-6725m.md` §6.4（帧级过渡表）、§10（全片对拍）
- 结论等级：机制结论 A 级（写点 PC + asm 行号双锚）；「Rust 是否在同 tick 内写过 09」已由复核证实（见下）
- **状态：2026-11-18 一轮独立复核 = reject（打点一/二 PASS、打点三 FAIL）⇒ 本文已按复核改判**，
  复核报告：`docs/rounds/r9-task-5a-transition-review.md`。历史主张一律原地保留 + 日期更正，不删除。

> 【2026-11-18 复核拒收 → 改判摘要】原裁定「Rust 房间请求晚约 2 帧、`F` 链与 Biz 整体 +2 平移」
> **不成立**，两处根因：
> ① **采样相位读错一帧**：`capture(N)` 的 `ram.hex` 是 **tick `N-1` 收口**
> （`crates/game-core/src/bin/rust_screenshot.rs:411` 原注释），上一版把它当 tick `N` 收口，
> 于是把 `capture(946)` 的值记到 f945 行，凭空读出 +2（复核 F4）；
> ② **「同 tick 覆写」分支已被证实**：`capture(945)=FF`、`capture(946)=F1`，且到 `F1` 必先经 `F0`
> （`EndOfRoom` 的 `stx #$F0`）⇒ `FF→09→F0→F1` 全在 **tick 945** 内完成，`09` 从不落帧边界（§5 第二行）。
>
> 新裁定：**不是缺写**；差异 = ①「ROM 的 `09` 存活 2 帧（驻留）未建模」为主 ②「请求 +1 帧」为次
> ③ `F` 链步进节律不同。**「采样相位」只解释那个 +2 的读数假象，不解释行为差异。**
- 本文不改任何游戏逻辑、不改阈值、不改门禁

## 1. 问题

前 1000 帧对拍里，`room`（`$005A`）出现两处失配：

| 帧 | Rust | BizHawk(quickerNES) |
|---|---|---|
| f27 | `FF` | `09` |
| f944 / f945 | `FF` | `09` |
| f948..f951 | `F1` / `F2` / `F4` / `FF` | `09` / `09` / `F1` / `F2`（见 §6.4 原表） |

要回答的是三件事，而不是「谁对」：
1. `$005A` 的取值序列由**谁**写（写点 PC/asm 行）；
2. `09`、`F0..F4`、`FF` 各自**能存活几个帧边界**（=能否被逐帧采样观测到）；
3. Rust 的差异属于「写错值」还是「写晚/写早」（时机）。

## 2. 方法：BizHawk 侧取证通道**不可用**（A 级负证据）

计划原定用 BizHawk Lua 的 `event.onmemorywrite` 记录 `$005A` 写序列。实测**该核心不支持内存回调**：

探针 `/tmp/probe_api2.lua`（注册 5 种回调，跑 120 帧，统计触发次数）：

```
event: availableScopes can_use_callback_params on_bus_exec on_bus_exec_any on_bus_read on_bus_write
       onconsoleclose onexit onframeend onframestart oninputpoll onloadstate onmemoryexecute
       onmemoryexecuteany onmemoryread onmemorywrite onsavestate unregisterbyid unregisterbyname
memory: ...（无 registerwrite / registerexec —— 2.10 该构建已移除旧式 hook）
onmemorywrite   reg ok=true  err=nil
on_bus_write    reg ok=true  err=nil
onmemoryexecuteany reg ok=true err=nil
onmemoryexecute reg ok=true  err=nil    (0xC0D3)
onmemoryread    reg ok=true  err=nil
mw=0 bw=0 ex=0 exa=0 mr=0          ← 120 帧内 5 种回调全部 0 次触发
```

`memory.registerwrite` / `memory.registerexec` 报 `attempt to call a nil value (field 'registerwrite')`；
> 【2026-11-18 更正·复核 F2】不是「2.10 移除了旧式 hook」，而是**这两个 API 名在 BizHawk 里从未存在**
> （它们是 FCEUX Lua 风格）；`monodis` 反编译整个 `BizHawk.Client.Common.dll` 中 0 次出现该名字。
> 另需限定：QuickNES **实现了 `ITraceable`**，所以正确的说法是「**Lua 内存回调通道**不可用」，
> 而不是「该核心无法被插桩取证」。

**`ok=true` 不是「注册成功」**：Lua 侧 `ok` 只表示 pcall 没抛异常，真实返回值是**空 GUID**
（`Guid.Empty`）。静态证据（复核方 `monodis` 反编译，见 `docs/rounds/r9-task-5a-transition-review.md` §1.2）：
`QuickNES::get_MemoryCallbacks` 标 `[FeatureNotImplemented]` 并 `throw NotImplementedException`；
`EventsLuaLibrary::OnBusWrite` 捕获该异常后返回 `Guid.Empty`。
另一路探针（只注册 `event.onmemorywrite`、300 帧内**不按地址过滤**）同样 `calls=0` ⇒ 不是地址域/参数顺序问题。

**结论**：`$005A` 写序列**无法**在 BizHawk(quickerNES) 上取证。按 D3（ROM 字节 + Mesen2 执行 = 最终真相）
改由 **Mesen2** 取证；BizHawk 只提供帧边界读数，两者在本文中显式区分。

## 3. ROM 写序列（Mesen2 实测，A 级）

用例（临时，已删）：

```
name tmp5a-write3
frames 1000
snapshot none
inputlog movies/eien86-6725M.inputlog
watch 005A RoomNumber
ctwrite 005A
ctwindow 930 960
endFrame 1000
```

`python3 tools/scenario_run.py run /tmp/tmp5a-write3.txt` → `analysis/traces/tmp5a-write3.ctwrite.log`（原始 12 行）：

```
[ctwrite] f=960 addr=5A val=09 pc=E758
[ctwrite] f=962 addr=5A val=F0 pc=EAF8
[ctwrite] f=963 addr=5A val=F0 pc=EA19
[ctwrite] f=963 addr=5A val=F1 pc=EA19
[ctwrite] f=964 addr=5A val=F1 pc=EA19
[ctwrite] f=964 addr=5A val=F2 pc=EA19
[ctwrite] f=964 addr=5A val=F2 pc=EA19
[ctwrite] f=964 addr=5A val=F3 pc=EA19
[ctwrite] f=965 addr=5A val=F3 pc=EA19
[ctwrite] f=965 addr=5A val=F4 pc=EA19
[ctwrite] f=966 addr=5A val=F4 pc=EA19
[ctwrite] f=966 addr=5A val=F5 pc=EA19
```

三个写点各自对得上 asm（`refs/m1disasm/SRC/prg7_engine.asm`）：

| PC | asm 锚 | 动作 |
|---|---|---|
| `E758` | `get_room_num` 径尾（同族 `:8082` `sta RoomNumber`，取 `WorldMapRAM $7000+(MapY*32+MapX)`） | 写入**新房间号** `09`（一次 `sta`） |
| `EAF8` | `:8878` `ldx #$F0 / stx RoomNumber`（`EndOfRoom ; 07:EAF4`，`EAF8`=`stx` 处） | 写 `F0`（房间写完） |
| `EA19` | `:8673-8675` `lda RoomNumber / and #$0F / inc RoomNumber`（`AttribTableWrite ; 07:EA13`，`EA19`=`inc` 处） | **每次 +1**，逐帧推进 `F0→F1→…→F4→(F5)` |

`F4→F5` 后由 `RoomFinished`（`:8687` `lda #$FF / sta RoomNumber`）落 `FF`（该窗口被 `ctwindow`
截断，`FF` 由帧级 `watch` 在 f952 观测到）。

**关键语义**：`CheckUpdateNameTable`（`:7703` `jsr SetupRoom`，`07:E54A`）每帧调一次 `SetupRoom`，
`AttribTableWrite` 每次调用只推进一格（`and #$0F` → `JumpEngine` 5 表项：
`ExitSub / WritePPUAttribTbl / ExitSub / WritePPUAttribTbl / RoomFinished`）。
游戏内三处过渡（f482 / f672 / f946 窗口）实测**每帧一格**（§4 读数），故 `F0..F4` 逐帧可观测。

> 【2026-11-18 更正】上一版此处写成「`inc RoomNumber` 每帧只执行一次」，**过强**：boot 段
> 同一条 `EA19` 在一帧内连推 4 格（上表原文 `f=40` 有 `F0→F1→F2→F3→F4` 共 4 次 `inc`）。
> 正确表述：**`EA19` 的推进速率随相位变化**——boot（无滚动/无 PPU 等待）一帧多格，
> 游戏内一帧一格。该差别本身是「核/相位」现象，不是状态机分支。

### 3.1 帧轴标定（必须写明，否则读数会错位）

`ctwrite` 的 `f` 与 `watch` 的 `f` **不同轴**：同一事件在 ctwrite 记 `f=960`、在 watch 记 `f=946`，
偏移恒为 **+14**（boot 段同族事件：ctwrite `f=38` ↔ watch `f=24`，同为 +14）。
本文所有结论都用 **watch 轴**（= 与 BizHawk 的 `f` 同语义：一回 `frameadvance` 一行）。

## 4. 三方帧级对照（watch 轴）

| f | Mesen2 `$5A`（watch，帧边界） | BizHawk `$5A`（帧边界） | Rust `$5A`（seam = pre-work；capture(f+1) = **tick f 收口**） |
|---|---|---|---|
| 941 | `09` 前身：`FF` | `FF` | `FF` |
| 944 | `FF` | **`09`** | `FF` |
| 945 | `FF` | **`09`** | `FF`（tick-close 已 `F1`） |
| 946 | **`09`** | `F1` | 无行（seam） |
| 947 | **`09`** | `F2` | 无行（seam） |
| 948 | `F0` | `F4` | **`F1`** |
| 949 | `F1` | `FF` | **`F2`** |
| 950 | `F3` | `FF` | **`F4`** |
| 951 | `F4` | `FF` | **`FF`** |
| 952 | `FF` | `FF` | `FF` |

> 【2026-11-18 更正·复核 F4】上一版把 `capture` 列说成「tick `f-1` 收口」，**错位一帧**：实际
> `capture(N)` = `seam(N)` = tick `N-1` 的 **pre-work**，而 `capture(N+1)` 才是 tick `N` 的 **post-work 收口**。
> 因此下文凡引用「f945 capture = F1」都应读作「tick 944 收口 = F1」。这条错位直接导致上一版把
> 「请求 +1 帧」误读成「+2 平移」（见 §5 更正）。

**读数**（三条独立事实）：
1. **`09` 确实被写过**，且在 ROM 侧可存活 **2 个帧边界**（Mesen 946/947；Biz 944/945）。
2. `09 → F0` 由 `EndOfRoom` 完成，`F0` 只在少数核/相位上落到帧边界（Mesen 948 有 `F0`，Biz 全程未见 `F0`）。
3. `F0 → F1 → … → FF` 的推进**每帧一格**，三方的格数序列不完全一样：
   - Mesen：`F0,F1,F3,F4,FF`（跳 `F2`：`F1→F2→F3` 在**同一帧内**完成了两次 `inc`，见 §3 原文 `f=964` 有两组 `inc`）
   - Biz：`F1,F2,F4,FF`（跳 `F0` 与 `F3`）
   - Rust：`F1,F2,F4,FF`（与 Biz 同形）

⇒ **`F` 序列的「跳格」是每帧 `inc` 次数不同造成的采样假象，不是状态机差异**；三方的值集合一致
（`{09, F0..F4, FF}`），只是每帧落点不同。

### 4.1 跨核帧轴标定（**限定 §4 的用法：Mesen 不能当帧级仲裁者**）

同一 `inputlog`、`snapshot none` 下，Mesen2 与 quickerNES 的同类转移逐个配对（同一脚本
`/tmp/tmpaxis.txt` 跑 1000 帧，取 `SamusStatus` / `RoomNumber` 转移点）：

| 事件 | Mesen f | BizHawk f | 跨核 Δ |
|---|---|---|---|
| status `0→$20` | 28 | 30 | **−2** |
| room `→09`（boot） | 24 | 26 | **−2** |
| status `21` | 166 | 166 | 0 |
| status `22` | 266 | 266 | 0 |
| status `23` | 356 | 356 | 0 |
| status `24` | 426 | 426 | 0 |
| status `→FF` | 466 | 466 | 0 |
| status `→01` | 467 | 467 | 0 |
| room `→23` | 482 | 482 | 0 |
| room `F1/F2/F4/FF` | 484/485/486/487 | 484/485/486/487 | 0 |
| room `→8` | 672 | 673 | −1 |
| room `→F0` | 675 | 676 | −1 |
| status `→00` | 953 | 952 | **+1** |
| status `→03` | 954 | 953 | **+1** |
| room `→09`（门过渡） | 946 | 944 | **+2** |

**结论（关键限定）**：两核帧轴**只在 f166..f487 段完全对齐**；boot 段差 2 帧，f672 起漂移到 1 帧，
f944 窗口漂到 2–3 帧。⇒
1. §4 的三方并列表里，**Mesen 列只能当「值集合/写点」证据，不能当帧级仲裁**；
   帧级主张只在 **BizHawk ↔ Rust** 之间成立（该轴由 `$0014` 锁存 967/967 全同钉死，见
   `movie-bizhawk-6725m.md` §6.3）。
2. 因此「Rust 房间请求晚 ~2 帧」这句必须读作：**相对 BizHawk 轴晚 ~2 帧**（$0014 锁轴），
   而不是「相对 ROM 真值晚 2 帧」——ROM 真值的帧级位置被两核漂移模糊了。
3. 跨核互证（计划 Task 5）**不能用全局偏移**，必须用局部锚事件（同一房间转移）对齐后再比。


## 5. 裁定

| 问题 | 裁定 | 依据 |
|---|---|---|
| Rust 是否**写错**房间号 | **否** | 三方 `$5A` 取值集合一致；Rust 也在 `F0→F1→F2→F4→FF` 推进（`crates/game-core/src/world/room_draw.rs:720,826,851`） |
| Rust 的 `f944/945 = FF` 而 BizHawk `= 09` | **时机/结构差**：请求 **+1 帧**（Rust tick 945 ↔ Biz frame 944）**+ ROM 的「`09` 存活 2 帧」驻留未建模** | BizHawk `09` 落 944/945；Rust tick 944 收口仍是 `FF`、tick 945 收口已是 `F1` ⇒ `09` 只存在于 tick 945 内部（⇒ 下一行） |
| Rust 的 `09` 是否「没写」 | **已证：写了，但从不在帧边界**——`FF→09→F0→F1` 在**同一 tick 内**折叠完成 | 反向推理：`capture(946)=F1` 而 `capture(945)=FF`；`F1` 的前态必是 `F0`（`room_draw.rs:826` 每步 `+1`），`F0` 只能由 `EndOfRoom` 的 `stx #$F0`（`prg7_engine.asm:8878` / `room_draw.rs:720`）产生 ⇒ tick 945 内必然走完 `09→F0→F1`。复核方独立复核同一结论：`docs/rounds/r9-task-5a-transition-review.md` §3 |
| Rust `F1..FF` 与 Biz 的关系 | **不是平移**：是**步进节律不同**——Rust 在 `F1` 停 3 个 tick；Biz 一帧一档（`F1/F2/F4/FF`） | 上一版「f948..f951 = f946..f949 的 +2 平移」是把「seam 比 Biz 边界慢 1 帧 + 946/947 无 seam 行」两处叠加误读成 +2（F4 更正）；真正的差是节律+到达时刻 |
| 哪个是「基准」 | `09` 由 ROM 写、且能被帧边界观测 ⇒ **Rust 缺的是「`09` 存活 2 个帧边界」这条驻留**（结构与节律），不是值、也不只是相位 | §3（`09@f960 → F0@f962` 真 2 帧驻留）+ §5 第二行 |

### 5.1 三分类

- **缺写**：无证据支持（值集合一致）。
- **结构差（主因，A 级）**：ROM 的「`09` 存活 2 个帧边界再 `EndOfRoom`」这条**驻留**未在 Rust 建模，
  ⇒ `09` 与 `F0/F1` 在同一 tick 内折叠，帧边界永远看不到 `09`（§5 第二行已证）。
- **时机差（次因，B→A 级）**：请求本身 **+1 帧**（tick 945 vs Biz frame 944）。
- **采样相位（仅解释读数，不解释行为）**：上一版的「+2 平移」是 seam/capture 差 1 帧 + 空洞跨 2 帧的读数假象；
  `F` 链「跳格」则是各核每帧 `inc` 次数不同的采样结果（见 §4 读数 3）。
  **本裁定不再声称「Rust 整体晚 2 帧」。**

### 5.2 反证条件（谁能推翻本裁定）

1. ~~若能在 Rust 侧证明 `get_room_num` 径在 f944 当帧就写了 `09`，则「请求晚 2 帧」降级为「相位差」。~~
   **【2026-11-18 已证伪/已闭合】** 该分支由复核方用既有 Rust capture 证实：`09` 与 `F0/F1` **同 tick**
   （tick 945），从未落帧边界 ⇒ 既不是「没写」，也不是「晚 2 帧」，而是「请求 +1 帧 + 驻留同 tick 折叠」。
   仍需 A 级钉死的唯一缺口：**Rust 写 `09` 的 tick 号**（需 `rust_screenshot --trace ram 005A` 启动参数），
   设计见 `docs/rounds/r9-task-5a-transition-review.md` §3.4。
2. 若 Mesen2 与 BizHawk 在同一 `inputlog` 下的**帧轴**存在系统性 ±2 偏移（本文件已见 boot 段
   Mesen `24=09` vs Biz `26=09`），则 §4 的跨核对照需先做帧轴标定再比较；此风险已登记（见 §6）。

## 6. 未做 / 已知缺口（不得当作已证）

1. **未做**：Rust 侧 `$005A` 写序列插桩（需要新增 `rust_screenshot --trace-room` 类启动参数；属代码改动，未获批）。
2. **已完成**：跨核帧轴标定（§4.1：boot `−2`、中段 `0`、门过渡 `+2`；复核方独立复现，见
   `docs/rounds/r9-task-5a-transition-review.md` §5）。**剩余未做**：漂移的**原因**——两核对帧边界/掉帧的
   定义不同（疑在 PPU vblank 等待与 `WritePPUAttribTbl` 的逐帧预算），属 Stage A 核心差异，不在本次范围。
   因此 §4 的跨核对照一律按「相对窗口位移」读，不作绝对帧对齐。
3. **未做**：`WritePPUAttribTbl` 的 vblank 等待为何在不同核上每帧推进格数不同（PPU 时序差，属 Stage A 核心差异）。
4. 本文件的 `ctwrite` 证据来自 Mesen2；BizHawk 侧只有帧边界读数（证据等级不同，已在 §2/§3 标明）。

## 7. 复现命令

```bash
# ① BizHawk 侧负证据（回调不可用）
cd /tmp/bh && ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy timeout 120 xvfb-run -a \
  ./EmuHawkMono.sh --chromeless --lua /tmp/probe_api2.lua \
  --movie "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" \
  "/home/peng/workspace/metroid-decomp/rom/Metroid (USA).nes"
cat /tmp/probe-api2.log        # mw=0 bw=0 ex=0 exa=0 mr=0

# ② Mesen2 写序列（$005A）
cat > /tmp/tmp5a-write3.txt <<'CASE'
name tmp5a-write3
frames 1000
snapshot none
inputlog movies/eien86-6725M.inputlog
watch 005A RoomNumber
ctwrite 005A
ctwindow 930 960
endFrame 1000
CASE
python3 tools/scenario_run.py run /tmp/tmp5a-write3.txt
cat analysis/traces/tmp5a-write3.ctwrite.log

# ③ 帧级 $5A 序列（Mesen watch 轴）
python3 - <<'PY'
import json
rows=[json.loads(l) for l in open('analysis/traces/tmp5a-write3.frames.jsonl')]
prev=None
for r in rows:
    v=r['w'].get('RoomNumber')
    if v!=prev: print(r['f'], v); prev=v
PY
```

## 8. 复现命令

### 8.1 附录：`capture(N)` 与 `seam(N)` 相位的可复算片段（复核 F7）

`capture(N).ram.hex` = **tick `N-1` 收口**（`crates/game-core/src/bin/rust_screenshot.rs:411` 原注释），
故 `capture(N) == seam(N)`；946/947 没有 seam 行（该 tick 无采样缝），只有 capture 可读 ——
这正是必须用 capture 补这两帧的原因。

```python
import glob, json
biz = {json.loads(l)["f"]: json.loads(l) for l in open("analysis/traces/bizhawk-6725M.jsonl")}
for f in range(938, 956):
    g = glob.glob(f"/tmp/rustshots-full/*_f{f:05d}_d{f}.ram.hex")
    cap = int(open(g[0]).read().strip()[0x5A * 2:0x5A * 2 + 2], 16) if g else None
    print(f, "capture(tick N-1 收口)=%s" % ("%02X" % cap if cap is not None else "-"),
          "biz=%02X" % biz[f]["room"])
```

实测输出（2026-11-18，读 `/tmp/rustshots-full/*.ram.hex` + `analysis/traces/bizhawk-6725M.jsonl`）：

| f | Rust `capture(f)`（= tick `f-1` 收口） | BizHawk `$5A`（帧边界） |
|---|---|---|
| 938..944 | `FF` | `FF`（944 = `09`） |
| 945 | `FF` | `09` |
| 946 | `F1` | `F1` |
| 947 | `F1` | `F2` |
| 948 | `F1` | `F4` |
| 949 | `F2` | `FF` |
| 950 | `F4` | `FF` |
| 951 | `FF` | `FF` |
| 952..955 | `FF` | `FF` |

读法：`capture(945)=FF` 是 **tick 944 收口**，`capture(946)=F1` 是 **tick 945 收口** ⇒
`FF→09→F0→F1` 的折叠发生在 tick 945 内部；`09` 因此不可能出现在任何帧边界上。

> 前提：`/tmp/rustshots-full` 是**可再生**临时产物（`--capture` 段输出，不入库）；若已清理，
> 按本文 §7 的 `rust_screenshot … --capture` 命令重跑 938..955 段即可复算。

## 8. evidence 摘要（供 commit message 引用）

- BizHawk(quickerNES) 内存回调不可用：5 种注册全 ok、120 帧触发 0 次（`/tmp/probe-api2.log`）。
- Mesen2 `$005A` 写点：`E758`（写 `09`）、`EAF8`（`EndOfRoom` 写 `F0`）、`EA19`（`inc`，逐帧 +1）。
  asm 锚：`refs/m1disasm/SRC/prg7_engine.asm:8673-8675`、`:8878`、`:8687`、`:7703`；
  RAM 锚：`crates/game-core/src/ram_map.rs:216`（`$005A`）。
- Rust 写点：`crates/game-core/src/world/room_load.rs:57,202`、`world/room_draw.rs:720,826,851`。
- 裁定：**时机错（房间请求晚 ~2 帧；`F` 链随之整体 +2 平移）**；「缺写」无证据；`09` 缺失可能是同 tick 覆写（未证）。
