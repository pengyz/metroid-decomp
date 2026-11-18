# R9 · Task 5A 房间过渡裁定 · 独立复核（verifier，证伪立场）

- 复核对象：`docs/findings/room-5a-transition-verdict.md`
- **复审（第二轮）对象：commit `5f8ebf2c`（改判后版本）——final verdict 见 §10：pass-with-findings**；
  下面 §1–§9 是对 `b070e265` 及以前的裁决（verdict = reject），保留作历史。
- 复核时的实际版本：**HEAD `b070e265`**（2026-11-18 11:21:44，含实施方后补的 §4.1 跨核标定），**不是**任务卡写的 `0e4ba0bb`；`0e4ba0bb` 之后文档又被改过一轮（含 §4.1）。
- 工作树状态（复核期间）：`crates/game-core/src/samus/physics.rs`、`handler.rs`、`data/status/baselines/*`、`docs/findings/movie-bizhawk-6725m.md` 等由实施方修改中，本次复核**未触碰**任何这些文件。
- 环境：Mesen2 = `tools/mesen2/Mesen` + `tools/scenario_run.py`（串行，`analysis/.scenario.lock`）；BizHawk 2.10 mono = `/tmp/bh/EmuHawkMono.sh`（`xvfb-run -a` + `ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy`，exit 137 = 自杀，正常）；录像 `/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2`。
- 只读证据件：`analysis/traces/bizhawk-6725M-full.jsonl`（28,408 行，未改动）、`/tmp/rustshots-full/*.ram.hex`（capture）、`/tmp/rustcap/*_seam.ram.hex`、`/tmp/rust-states-t2.jsonl`（seam 状态行）。
- 本次自己的产物全部在 `/tmp/verify-5a-*`；跑完已把误落在 `analysis/traces/verify5a-axis-*` 与 `analysis/_gen/verify5a-axis-*.lua` 的 30 个文件移入 `/tmp/verify-5a-artifacts/`（仓库内已无残留）。

## verdict（第一轮，对象 `b070e265`）：**reject**

## 三行结论

1. **打点一（BizHawk 没有内存回调）= PASS**：我独立复现了负证据，且升级为「静态 IL + 运行时」双证——`QuickNES.get_MemoryCallbacks()` 方法体是 `throw NotImplementedException`，`EventsLuaLibrary.OnBusWrite` 捕获后返回**空 GUID**；但裁定书把判据写错了（`ok=true` 不是成功，**返回空 GUID 才是失败**），且「`memory.registerwrite` 在 2.10 被移除」是误述（该 API 名在 BizHawk 里从未存在）。
2. **打点二（ctwrite 与 watch 恒差 +14）= PASS**：三段（boot≈0、mid≈490、trans≈950）实测偏移都是 +14，§3.1 标定成立。
3. **打点三（同 tick 覆写 09 是否削弱「晚 2 帧」）= FAIL，且该分支已被现有证据证实**：Rust 在 **tick 945** 内完成 `FF→09→F0→F1`（`capture(946)=F1`），09 从未落在帧边界；ROM 侧 Mesen `ctwrite` 显示 **09@f960 → F0@f962**（真有两帧间隔，名表行流送跨帧），Biz 侧 09 存活 f944/f945 两个边界。故「Rust 房间请求晚约 2 帧」与「Rust 序列 = Biz 序列整体 +2 平移、形状完全一致」两条核心主张都不成立：实际是 **请求 +1 帧、2 帧驻留被同 tick 折叠、F 链步进形状不同**。裁定书用 seam（pre-work）列跨越 946/947 两行数据空洞去对齐 Biz 的 post-work 边界，系统性多读出 1 帧，`+2` 是相位+空洞造成的读数假象。

---

## §1 打点一：BizHawk quickerNES 内存回调不可用

### 1.1 我先打「是不是探针写错了」

读 `/tmp/probe_api2.lua`：它只 `pcall(fn)` 记 `ok/err`，**不取 API 返回值** —— 只要没抛 Lua 异常就打印 `ok=true`。这是判据缺陷，不是结论缺陷。

自写探针 `/tmp/verify-5a-bh-probe.lua`（注册前/后双相位、打印每个 API 的 pcall 结果**与返回值**、窗口内主动自写 `$5A` 作阳性对照、130 帧后复查）：

```
onmemorywrite_wildcard  pcall_ok=true  ret_type=string ret=00000000-0000-0000-0000-000000000000
onmemorywrite_5A        pcall_ok=true  ret=00000000-0000-0000-0000-000000000000
onmemoryexecute_wildcard pcall_ok=false "Invalid arguments to method call"
onmemoryexecute_C0D3    ret=00000000-...
onmemoryread_5A         ret=00000000-...
on_bus_write            ret=00000000-...
(5 帧后重注册)          ret=00000000-...
has_registerwrite=nil  has_registerexec=nil  has_emu_addmemorycallback=nil
hits_after_30f=0  self_write_hits=0  final_hits=0  framecount=130
```

阳性对照（自写 $5A）也 0 命中，排除「只是没触发」；5 帧后重注册同样空 GUID，排除「注册发生在 rom 加载前被覆盖」；`client.unpause()` 路径本来就在跑（framecount 增长到 130），排除「没进帧循环」。

### 1.2 静态确认（比实施方更强的一层）

`monodis` 导出 `/tmp/verify-5a-*.il` 后：

- `BizHawk.Emulation.Cores.dll` → `...Nintendo.QuickNES.QuickNES::get_MemoryCallbacks()`（方法 line 8222）带 `[FeatureNotImplemented]`，方法体 `newobj NotImplementedException; throw`（IL 632950-632972）。
- `BizHawk.Client.Common.dll` → `EventsLuaLibrary::OnBusWrite/OnBusRead` 捕获 `NotImplementedException` → `LogMemoryCallbacksNotImplemented` → **返回 `Guid.Empty`**；`onmemorywrite` 带 `LuaDeprecatedMethodAttribute`，转 `OnBusWrite`。
- `memory.registerwrite|registerexec` 在该 dll 的 IL 中**出现 0 次**（不是「2.10 移除」，是从来没有）。

### 1.3 但有一处需要收窄

`QuickNES` 还实现了 `ITraceable`（`TraceBuffer`/`TraceCallback`/`ConnectTracer`/`qn_set_tracecb`），即 BizHawk **Trace Logger 通道存在**。所以「$005A 写序列无法在 BizHawk 取证」不能写成整个核心无法取证，只能限定为「**Lua 内存回调通道**不可用」。

**判定：结论 PASS；判据/措辞 2 处需改（见 §7 F1、F2）。**

## §2 打点二：ctwrite ↔ watch 恒 +14

重建三份用例（`/tmp/verify-5a-axis-{boot,mid,trans}.txt`，`watch 005A RoomNumber` + `ctwrite 005A` 同开，`ctwindow` 分别 0 60 / 465 505 / 930 975），串行跑 `python3 tools/scenario_run.py run <file>`：

| 段 | ctwrite 首笔（E758 = 09） | watch 首个 09 边界 | 偏移 |
|---|---|---|---|
| boot | f=38 (09, pc=E758) | f=24 | **+14** |
| mid（room→0x17） | f=496 (17, pc=E758) | f=482 | **+14** |
| trans（room→09） | f=960 (09, pc=E758) | f=946 | **+14** |

另外 boot 段 `ctwrite f=38 5A←FF(pc=C869)`、`f=40 F0(pc=EAF8)`、`f=40 F0..F4(pc=EA19)`；trans 段 `f=960 09 / f=962 F0 / f=963..966 F0..F5` —— 与裁定书 §3 的 12 行逐值一致。

机制：`ctwrite` 的 `frame` 取 `emu.getState().frameCount`（`tools/scenario_capture.lua` ~419-450），`watch` 取 scenario 自己的 `inputPolled` 计数（~1120-1160），两者相差恒定的 vblank/帧计数起点（+14）。

注意一处采样细节：`ctwrite` 对**每个地址**最多 12 笔，F4→FF 那段会被截断（`RoomFinished` 落 FF 可能不在日志里），比较时要用 `watch` 补齐。

**判定：PASS。**

## §3 打点三：同 tick 覆写 09 —— 分支已被证实，核心主张不成立

### 3.1 先钉死采样相位（这是裁定书最容易错的一步）

`crates/game-core/src/bin/rust_screenshot.rs`：

- `state` 行在 `engine.tick_frame_with(...)` 闭包内 push ⇒ **seam = post-NMI / pre-work 的 tick f 开头**；
- `capture`（`*.ram.hex`）在循环 frame 头发射、且在 joy 注入之前 ⇒ **capture(N) = tick N-1 收口**（源码注释 411/488/489 自己就写了这一句）。

我独立读两个通道的 `$5A`：`capture(N) == seam(N)`（全等），即 **seam f ≡ capture f ≡ tick f-1 收口**。所以：

- `seam(f)` 是 **pre-work**，对「帧内变化量」$5A 比 BizHawk 的 post-frame 边界**系统性慢 1 帧**；
- 裁定书 §4 的 rust-close 列取值等于 `capture(f+1)`（我逐格核过：944 FF / 945 F1 / 946 F1 / 947 F1 / 948 F2 / 949 F4 / 950 FF ↔ capture 945..951 全等），**表头写「tick f-1 收口」是错位一帧**，实际是 **tick f 收口**。
- `$0014` 锁存对得上（`/tmp/rust-states-t2.jsonl` 的 `zp[0x14]` 与 Biz `joy1_latch` 在 f935..955 逐帧同值，f951=01、f952=00、f953=04），所以**轴号**是对齐的（Rust tick N ↔ Biz frame N）；但锁存在 NMI 帧首写，**只能证轴号、不能证 $5A 同相位** —— 裁定书用 967/967 去当「相位也已钉死」用，是过度解读。

### 3.2 换成同相位对齐（post-work ↔ post-work）

`capture(f+1)`（= tick f 收口）对 Biz 边界 f：

| f | Rust（tick f 收口） | Biz（frame f 后） | 读法 |
|---|---|---|---|
| 944 | FF | 09 | Rust 尚未写 09 |
| 945 | **F1** | 09 | Rust 已把 09 推到 F1；Biz 还停在 09 |
| 946 | F1 | F1 | 同 |
| 947 | F1 | F2 | Rust 慢 1 档 |
| 948 | F2 | F4 | 慢 2 档（步进差） |
| 949 | F4 | FF | 慢 1 档 |
| 950 | FF | FF | 同 |

结论三条，全部与裁定书相反或需改写：

1. **Rust 的 09 请求确实发生了，且在 tick 945**（tick 944 收口还是 FF，tick 945 收口已是 F1）。ROM 侧 Biz 是 frame 944 写 09 ⇒ **请求晚 1 帧，不是 2 帧**。
2. **09 是被同 tick 覆写的（已证实，不再「未证」）**：tick 945 内 `FF→09→F0→F1`，F1 要求先有 `EndOfRoom` 的 `stx RoomNumber`（0xF0）再 `inc`（0xEA19），而 tick 944 收口是 FF ⇒ 09 与 F0/F1 **必在同一次 tick**。裁定书 §5.2 的分支 1（「若 f944 当帧就写 09 ⇒ 降级为相位差」）在「同 tick 折叠」意义上**成立**。
3. **「形状完全一致、整体 +2 平移」不成立**：Rust 在 F1 停 3 个 tick（945/946/947 收口都是 F1），Biz 是 F1/F2/F4/FF 一帧一档。两者是**步进节律差**，不是平移。

### 3.3 ROM 侧的「两帧驻留」是真的，机制也已定位

Mesen `ctwrite`（同段、串行）：`09@f960(E758)` → **`F0@f962(EAF8)`** → `F0..F5@f963..966(EA19)`。09 与 F0 之间隔 **2 帧**。

asm 对齐（`refs/m1disasm/SRC/prg7_engine.asm`）：

```
7702 CheckUpdateNameTable: ; 07:E54A
7703     jsr SetupRoom          ; ← SetupRoom 内 get_room_num 尾写 09（:8082 sta RoomNumber）
7704     ldx RoomNumber
7705     inx
7706     bne RTS_X173           ; room != FF ⇒ 继续
...
7712     jmp CheckUpdateNameTableVertical
7714     jmp CheckUpdateNameTableHorizontal
```

即：**09 是 `SetupRoom` 写的；同一调用里接着做名表行流送（vertical/horizontal），直到滚动完成才由 `EndOfRoom`（:8875 / `ldx #$F0` :8877-8878）写 F0**。名表行流送是「按帧预算做一行」的活 ⇒ 自然占 2 帧。Rust 侧这段（`crates/game-core/src/world/camera/scroll.rs:13-80` 的 `check_update_name_table` + 列写核，以及 `world/room_draw.rs` 的属性链）在 tick 945 内一次跑完 ⇒ 折叠。这与 `room_load.rs` 自己那句注释「SetupRoom 环+名表拷贝=重活——原版掉帧窗 bogged 2 帧」正好互相印证：**掉帧窗没被建模**（`work_cycles` 累加了 `LOAD_SHELL/LOAD_OBJ/ROW/YLOOP...`，但没让这些工作溢出到下一帧）。

### 3.4 能区分两分支的最小实验（写清设计与预期观测值）

现有证据已足以排除「相位纯假象」分支（09 确实是同 tick 被覆盖，不是采样没抓到）。若要把「请求 +1 帧」也钉死到无可辩，最小实验如下（**不需要跑长段**，1 次 Mesen + 1 次 Rust 即可）：

- 用例：`frames 1000 / inputlog movies/eien86-6725M.inputlog / ctwrite 005A / watch 005A / ctwindow 930 975`，再加一条 `ctwrite 0014`（输入锁存）。
- Rust 侧：`rust_screenshot` 加 `--trace ram 005A`（一次性开 `$5A` 写日志，带 tick 号；**不要**改成生产钩子，走启动参数，见 AGENTS.md §3 第 8 条）。
- 期望观测（判定表）：
  - 分支 A「请求准时、只丢 2 帧驻留」：Rust 写 09 的 tick == Biz frame 944；则 §4 表在 f944 应看到 Rust 已有 09（而实际没有）⇒ **已被现有 capture 排除**（capture 945 = FF）。
  - 分支 B「请求 +1」：Rust 写 09 的 tick == 945（与 capture 946=F1 一致）⇒ **现有数据已支持**。
  - 分支 C「请求 +2」：tick 应为 946；但 capture 946 已是 F1（F1 需要 09 先写），**不可能**。
- 判据：`ctwrite 0014` 的锁存序列与 Rust seam 的 `zp[0x14]` 若在 944-950 逐帧同值，则轴号对齐成立，「tick 945 vs frame 944 ⇒ +1」成立；若错一帧，则改判为「同相位、请求准时」。这一步是把 §3.2 的 +1 从「高置信」升到「A 级」的唯一缺口。

**判定：FAIL（裁定书该条的核心量化结论不成立；术语应改，见 §7 F3/F4）。**

## §4 锚点核对（机械可验证项逐条）

| 裁定书锚点 | 实际 | 结论 |
|---|---|---|
| `prg7_engine.asm:8673-8675` `lda RoomNumber / and #$0F / inc RoomNumber` | 8672 `AttribTableWrite:`；8673 `lda RoomNumber`；8674 `and #$0F`；8675 `inc RoomNumber` | **PASS（逐字对）** |
| `:8878` `stx #$F0` | 8875 `EndOfRoom:`（07:EAF4）；8877 `ldx #$F0`；8878 `stx RoomNumber` | PASS（行号落在 `stx`，写法「`stx #$F0`」不严谨，`#` 在 8877） |
| `:8687` RoomFinished 落 FF | 8685 `RoomFinished:`；8686 `lda #$FF`；8687 `sta RoomNumber` | **PASS** |
| `:7703 CheckUpdateNameTable` | 7702 `CheckUpdateNameTable:`；7703 `jsr SetupRoom` | PASS（标签在 7702，`jsr` 在 7703） |
| `crates/game-core/src/ram_map.rs:216` = `$005A` | `pub const ROOM_NUMBER: u16 = 0x005A` | **PASS** |
| `room_load.rs:57` | `mem.set_room_number(0xFF);` | PASS |
| `room_load.rs:202` | `mem.set_room_number(room);` | **PASS** |
| `room_draw.rs:720` | `mem.set_room_number(0xF0);` | **PASS** |
| `room_draw.rs:826` | `set_room_number(room.wrapping_add(1))`（inc） | PASS |
| `room_draw.rs:851` | `set_room_number(0xFF)`（RoomFinished） | PASS |
| Mesen 写点 `E758/EAF8/EA19` = get_room_num 尾 / EndOfRoom / AttribTableWrite | asm `8082 sta RoomNumber`（get_room_num 尾）、`EndOfRoom`、`AttribTableWrite` 一一对应；ctwrite 报告的 PC 是回调时刻 PC | PASS |

**锚点整体 PASS**，无需改结论，只有 §7 F6 两处行号写法的小订正。

## §5 跨核轴：§4.1 是对的，§6.2 与它相互矛盾

我独立用 Mesen 全段 watch（`/tmp/tmpaxis.jsonl`，实施方 11:13 产物）与 Biz 全段对比，复算四点：

| 事件 | Mesen | BizHawk | 偏移 |
|---|---|---|---|
| boot room →09 | 24 | 26 | **−2** |
| status →$20 | 28 | 30 | **−2** |
| status 21/22/23/24/FF、room→0x23、F 链 | 166/266/356/426/466/482 全同 | 同 | **0** |
| 房门过渡 room →09 | 946 | 944 | **+2** |
| status →00 / →03 | 953/954 | 952/953 | **+1** |

⇒ 两核相对轴在片内**漂移 4 帧**（boot −2 → 过渡 +2）。所以：

- §4.1「帧级主张只在 BizHawk↔Rust 之间成立、Mesen 只作值集合/写点证据」**正确**，我复现了它的每个数；
- §6.2 仍写「未做跨核帧轴标定」**与 §4.1 直接打架**（P2，见 F5）；
- 附带一个实施方没写的事实：在 boot 锚点上 **Rust 与 Mesen 同相（28）而比 Biz 早 2**，在过渡锚点上 Rust 比 Mesen 早 1、比 Biz 晚 1 —— 「晚 2 帧」这个数字**换个核就读不出来**，这本身就是「它是个相位读数而不是物理量」的独立佐证。

## §6 实跑命令与关键输出（全部串行，未跑 cargo 全量、未跑 l4）

```bash
# 打点一：BizHawk 回调（自写探针，含返回值捕获）
cd /tmp/bh && ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy timeout 180 xvfb-run -a \
  ./EmuHawkMono.sh --chromeless --lua /tmp/verify-5a-bh-probe.lua \
  --movie "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" \
  "/home/peng/workspace/metroid-decomp/rom/Metroid (USA).nes"     # exit 137 = 正常自杀
# 关键行：见 §1.1；日志 /tmp/verify-5a-bh-probe.log

# 打点一的静态证据
monodis --output=/tmp/verify-5a-common.il /tmp/bh/dll/BizHawk.Client.Common.dll
monodis --output=/tmp/verify-5a-cores.il  /tmp/bh/dll/BizHawk.Emulation.Cores.dll
# QuickNES::get_MemoryCallbacks → [FeatureNotImplemented] + throw NotImplementedException
# EventsLuaLibrary::OnBusWrite → catch NotImplementedException → Guid.Empty

# 打点二/§5：Mesen 三段同开 ctwrite+watch（串行）
python3 tools/scenario_run.py run /tmp/verify-5a-axis-boot.txt    # DONE trace_lines=1000
python3 tools/scenario_run.py run /tmp/verify-5a-axis-mid.txt
python3 tools/scenario_run.py run /tmp/verify-5a-axis-trans.txt
# 关键行见 §2 表；产物已移至 /tmp/verify-5a-artifacts/

# §3：Rust 采样相位与 $5A 序列（只读既有产物，未跑 cargo）
python3 - <<'PY'   # capture(N)==seam(N)；capture 945=FF,946=F1,948=F1,949=F2,950=F4,951=FF
... 读 /tmp/rustshots-full/*.ram.hex + /tmp/rust-states-t2.jsonl ...
PY
# 输出：f938-945=FF / f946..947 无 seam 行 / f948 F1 / f949 F2 / f950 F4 / f951 FF
# 锁存对齐：rust seam zp[0x14] == biz joy1_latch（f935..955 全等）
```

附：`analysis/traces/bizhawk-6725M-full.jsonl` 全程只读，未写、未 `cmp` 之外的操作，未改动。

## §7 需改条目（按严重度）

**F1（P0，判据错）** §2 把「`ok=true`」当注册成功。修法：改写成「**返回值是空 GUID** ⇒ 核心未实现回调」，并引用 `EventsLuaLibrary::OnBusWrite` 的 catch 分支与 `QuickNES.get_MemoryCallbacks` 的 `[FeatureNotImplemented]`；`ok=true` 只是「Lua 没抛异常」的 pcall 语义。

**F2（P1，事实错）** 「`memory.registerwrite/registerexec` 在该构建移除」。修法：改为「该 API 名在 BizHawk 里**从未存在**（FCEUX 风格 API），IL 中 0 次出现」，并补一句限定：QuickNES 实现了 `ITraceable`，所以是「**Lua 内存回调通道**不可用」，不是「核心无法取证」。

**F3（P0，结论错）** §5 的「Rust 的房间请求**晚约 2 帧**」与「Rust 序列 = Biz 序列**整体 +2 平移、形状完全一致**」。修法（用 §3.2 表替换）：
- 请求：**tick 945**（Biz frame 944）⇒ **+1 帧**；
- 09 缺失：**同 tick 折叠**（`FF→09→F0→F1` 一次 tick 内做完），09 从未落边界 —— 这一分支应从「未证」升级为「已证」；
- F 链：**步进节律不同**（Rust 在 F1 停 3 tick；Biz F1/F2/F4/FF 一帧一档），不是平移；
- 分类改判为「**不是缺写；是时机/结构差：2 帧驻留未建模（主）+ 请求 +1 帧（次）**，其中「采样相位」只解释**表里那个 +2 的读数**，不解释行为差异」。

**F4（P0，相位标定错）** §4 表头「rust-close = tick f-1 收口」与桥接说明错位一帧（实际 `capture(f+1)` = tick f 收口）；`$0014` 967/967 只能证**轴号**对齐，不能证 `$5A` 同相位。修法：表头改为「tick f 收口」，并在表下加一行「seam(f) = pre-work tick f = post-work tick f-1；对帧内变化量，seam 比 Biz 边界慢 1 帧，跨 946/947 空洞比较容易多读 1 帧」。

**F5（P2，自相矛盾）** §6.2「未做跨核帧轴标定」与 §4.1 并存。修法：删/改 §6.2，写明「已完成：§4.1；剩余未做：跨核 ±2 漂移的**原因**（哪个核的帧边界定义/掉帧策略不同）」。

**F6（P2，行号写法）** `:8878 stx #$F0` → 「`:8877 ldx #$F0` / `:8878 stx RoomNumber`」；`:7703 CheckUpdateNameTable` → 「`:7702` 标签 / `:7703 jsr SetupRoom`」。

**F7（P3，缺证据指针）** 两处「09 未证」「F 链跳格」都缺可复现的读取脚本。修法：把 `capture(N)==seam(N)` 的核对与 §3.2 的表用一条 `python3 -` 片段写进文档附录（读 `/tmp/rustshots-full` + `/tmp/rust-states-t2.jsonl` + biz jsonl），让审稿人能一键复算。

## §8 若按「时机/结构差」定案，Rust 侧下一步改哪里（只给定位线索，不改代码）

1. **主因：2 帧驻留没建模**。ROM 的路径是 `CheckUpdateNameTable: jsr SetupRoom`（`prg7_engine.asm:7702-7714`）→ 09 写入 → **同一调用继续做名表行流送（按帧预算一行）** → 2 帧后才 `EndOfRoom` 写 F0。Rust 的对应件：
   - `crates/game-core/src/world/camera/scroll.rs:13-80`（`check_update_name_table` + 列写核）—— 检查它是否把「一帧一行」的节律压成一次跑完；
   - `crates/game-core/src/world/room_draw.rs:720`（`EndOfRoom` 写 F0）与 `:826/:851`（attrib inc / RoomFinished）的相对时序 —— 现在与 `room_load.rs:202` 的 09 写在**同一 tick**；
   - `crates/game-core/src/frame/cycles.rs`（掉帧窗模型 `dropped = (BASE + work_cycles) / FRAME_CYCLES`）与 `room_load.rs:212` 的 `LOAD_SHELL`、`room_draw.rs:113/117/155/169/645/808` 的 `LOAD_*` —— 现有累加是否**真正触发溢出到下一帧**（本过渡里没有触发，就是折叠的直接原因）。
2. **次因：请求 +1 帧**。若 §3.4 的 `ctwrite 0014` 对照实验确认轴号对齐，则 `room_load.rs:202` 的 09 写点比 ROM 晚 1 tick —— 线索在触发条件（ScrollBlock 门/`get_room_num` 的调用时机，`room_load.rs` 180-230），不要在值上找。
3. **不要改的**：`RoomNumber` 的任何写值（09/F0/F1/FF 都对）、`ram_map.rs` 的 `$005A`、`room_draw.rs:851` 的 FF 写法 —— 本复核已核对为 A 级正确。

---

## §9 追加复核：Task 5 跨模拟器互证（`movie-bizhawk-6725m.md` §11 + `tools/movie_ref/compare_cores.py`，2026-11-18 11:36）

复核时两件均**未提交**（§11 在工作树、`compare_cores.py` 未跟踪）。工具本体我读过 178 行：schema 映射、`--shift` 语义、`--json-out` 都对；`--latch-align` 的 `best=(0,-1.0)` 初值在「无可比帧」时会静默返回 `shift=0 rate=-1`（无害但应 assert `n>0`）。

### 9.1 四点逐个结论

| # | 实施方主张 | 我的复算 | 判定 |
|---|---|---|---|
| 1 | `$0014` 反解 **shift=0**（一致率 0.9786）；`--shift 1/2` 更差（27/36 失配帧） | 一致率 **0.9786**、最优 shift=**0** 复现；shift=1 → **24 帧**失配（16 room + 9 status）、shift=2 → **36 帧** | **PASS（弱形式）**；`27` 这个数复算不到（见 9.2 P3） |
| 2 | `--start 40 --end 1000 --shift 0` → `health 0 / scrolldir 0 / status 2 / room 13`，分组 `{"attrib-672":5,"attrib-944":10}` | 逐字复现（15 个失配帧，f672/675/676/677/678 + f944..953，全部在窗口内） | **PASS** |
| 3 | Mesen 侧 `/tmp/tmpaxis2.jsonl` 可复现 | 我用 §11.6 的用例**重跑一遍**（30.9s），与 `/tmp/tmpaxis2.jsonl` **1000 帧逐帧 0 差异** | **PASS** |
| 4 | 链成员：Mesen `8→F0→F1→F3→F4→FF` vs Biz `8→F0→F2→F3→FF`（f672）；`9→F0→F1→F3→F4→FF` vs `9→F1→F2→F4→FF`（f944） | 逐行取值全对（f672..679：Biz `FF,8,8,8,F0,F2,F3,FF` / Mesen `8,8,8,F0,F1,F3,F4,FF`；f944..952 两侧同上） | **PASS** |

### 9.2 但「shift=0 ⇒ f≥40 同帧号可比」这个强形式不成立（P1，需改 §11.2/§11.3）

工具只支持**全局单一 shift**，而实测相对相位是**逐窗口不同**的：

| 窗口 | room 失配最优 shift | 该 shift 下剩余失配 | shift=0 失配 |
|---|---|---|---|
| attrib-482（f482..487） | **0** | 0 | 0 |
| attrib-672（f672..679） | **+1** | 2 | 5 |
| attrib-944（f944..953） | **−2** | 4 | 8 |

（语义 `biz f ↔ mesen f-shift`。全段 f40..1000 的全局最优仍是 shift=0：room 13 / status 2，而 −1 是 18/6、+1 是 16/9、±2 更差 —— 这就是 0.9786 的来历。）

两条推论，§11 必须写进去：

1. **「f≥40 起同帧号可比」要改成「非过渡窗口同帧号可比」**。attrib-944 窗口里 Mesen 整体晚 2 帧（f944/945 Mesen=FF、Biz=09；f946/947 Mesen=09、Biz=F1/F2），这不是采样相位而是**两核到达该过渡的时刻差 2 帧**（与我在 §5 独立测得的 boot −2 / 过渡 +2 漂移一致）。
2. **「中间值不可作帧级证据」的归因要改**：即使在最优 shift=−2 下，attrib-944 仍剩 4/10 room 失配 ⇒ 两核在该窗口的**写次数/帧**真的不同（Biz 一帧一格 F1/F2/F4，Mesen 有单独 F0 帧 F0/F1/F3/F4），这是核间执行差异，不是「采样相位」。词要改成「两核在该窗口的 attrib inc 推进速率与到达时刻不同，中间值不可比，只比端点」。

### 9.3 另外三个可执行修法

- **P2（工具口径未写明）**：`group()` 里 `attrib-482/672/944` 是**硬编码窗口**，「15 帧全在窗口内」这句话有循环论证的气味（虽然我复算确实没有窗口外失配）。修法：§11.3 加一句「窗口定义写在 `compare_cores.py:group()`；失配帧集合独立于分组，窗口外确为 0」。
- **P3（数字对不上）**：`shift=1 → 27 失配帧` 复算为 **24 帧 / 25 字段**。修法：用 `--show 0` 重抄一遍输出，别手算。
- **P2（`$0014` 不是干净的标尺）**：Biz 的 `joy1_latch` 有 **57 帧**落在 `{0,1,2,130}` 之外（f0-6 是上电 `$FF` 填充；游戏内零散单帧 662/664/666/668/670-672/686/953/1031/1080/1162…），Mesen 同帧没有这些值（例：f953 Biz=9、Mesen=0；f662 Biz=3、Mesen=2）。⇒ 该字段在两核**采样相位不同/有撕裂**，`0.9786` 被这类毛刺压低，**不能当作「轴严格同帧」的证明**，只能当「大体 0 偏移」。修法：§11.2 加这一句；若要更硬的轴标定，用 `$0300` 状态转移 + `$005A` 端点两路交叉（我已做，见 §5 表）。

### 9.4 与本文 §3 的交叉矛盾（必须同步改，P0）

§11.4 第 3 条写「Rust 从未在 tick 收口相位出现过 `09` ⇒ …（但**仍未证**是否被同 tick 覆写，见 §5.2 反证条件 1）」。**这一句已被本文 §3 证实**：Rust `capture(946)=F1`（tick 945 收口），而 tick 944 收口是 `FF`，F1 必须先有 `EndOfRoom` 的 `stx #$F0` ⇒ `09` 与 `F0/F1` **必在同一 tick**。两份文档不能一个写「已证」一个写「未证」，`movie-bizhawk-6725m.md` §11.4-3 与 `room-5a-transition-verdict.md` §5.2 都要改成已证（并把「晚 2 帧」改为「请求 +1 帧 + 2 帧驻留同 tick 折叠」）。

### 9.5 §11.5「未做」是否足以支撑 §11.4「基线可迁移」

- `f>1000`、像素/音频面：**不足以**推翻弱结论，可保留为未做。
- 「为什么相位差 1~2 帧的时序考证」：**我判为必须做**，否则 §11.4-1 只能写成「非过渡窗口可迁移、过渡窗口只比端点」，不能写成无条件可迁移。最小做法已写在本文 §3.4（同一条 `ctwrite 0014` 对照即可同时服务两处）。

---

## §10 复审（`5f8ebf2c`，2026-11-18 11:50）：改判成立 → **pass-with-findings**

- 对象：commit `5f8ebf2c`（只改 `docs/findings/room-5a-transition-verdict.md`，+88/−17，314 行）。
- 方式：**只读**。未跑模拟器长跑（复用本轮 §2/§3 的实测件与 `/tmp/rustshots-full/*_d*.ram.hex`），
  只跑 python 读 hex / glob 与 `git show`。
- 总判：**核心改判成立**（不是缺写；「`09` 存活 2 帧的驻留未建模」为主因、「请求 +1 帧」为次因、
  `F` 链不是 `+2` 平移），两处「加强」我都独立复算通过。但文件内部仍有 **2 条 P1 自相矛盾**
  （旧裁定残留在 §8 evidence 摘要、§4 Rust 列仍错位一帧）+ **2 条 P2** + **5 条 P3**，
  需一次更正提交 ⇒ **pass-with-findings**。

### 10.1 打点 1：新 §5 的三成分拆分 —— PASS（但有 2 处必须补的量化）

我用自己的 capture 读数做闭合验算（`capture(N)` = tick `N-1` 收口，本报告 §3.1 已钉死；
故 tick `k` 的收口 = `capture(k+1)`）：

| tick `k`（post-work） | Rust 收口（=`capture(k+1)`） | BizHawk 帧 `k` 边界 | 相对偏移 |
|---|---|---|---|
| 944 | `FF` | **`09`** | Rust 尚未写（09 驻留缺） |
| 945 | **`F1`** | **`09`** | Rust **早于** Biz（折叠抢先） |
| 946 | `F1` | `F1` | 0 |
| 947 | `F1` | `F2` | +1 |
| 948 | `F2` | `F4` | +2 |
| 949 | `F4` | `FF` | +3 |
| 950 | `FF` | `FF` | 0 |

结论：

1. **三成分足够做分类**，不需要「第四条」。我核过最有威胁的候选「`F0` 也不落帧边界」：
   `F0` 只在 **Mesen** 落边界（948），Biz 与 Rust 都从不落 ⇒ 它是核间现象，不是 Rust-vs-Biz 成分
   （§4 读数 2 已写对）。**boot 段同机制**也成立：粗档 capture（`_cap` 文件，f20..170 每 10 帧）
   `$5A` 恒 `FF`，从未见 `09` ⇒ 不是 f944 单点现象。
2. **但 ③「步进节律」欠分解**：真实形状是「**attrib 首步（`F1→F2`）延迟 2 tick**」+「其后 1 tick/格」，
   不是均匀慢（Rust：F1@945→F2@948→F4@949→FF@950；Biz：F1@946→F2@947→F4@948→FF@949）。
   建议 §5 第三成分改写为这两条，并在 §6.3 把「首步延迟 2 tick」点名为待查
   （候选：该 tick 的 `work_cycles` 预算提前耗尽 ⇒ 当帧没调 `check_update_name_table`，
   见 `crates/game-core/src/world/camera/scroll.rs:13-80`、`room_load.rs:212` 的 `LOAD_SHELL`）。
3. **必须补进 §5 的反直觉事实**：Rust 到 `F1` 比 Biz **早 1 tick**（945 vs 946）。若只写「请求 +1 帧」，
   读者会推断「Rust 单调落后」，与实测相反——偏移序列是 `0/−1/0/+1/+2/+3/0`（见上表）。
4. **「请求 +1 帧」的证据等级三处口径不一**：§5 表格裸断言、§5.1 写「B→A 级」、§5.2 说唯一缺口是
   「Rust 写 `09` 的 tick 号」。建议统一成一句：**A 级（边界反推 + 唯一写点）**——
   `capture(945)=FF` ∧ `capture(946)=F1`，且 `$5A←F0` 只有 `room_draw.rs:720`、`$5A←新房间号`
   只有 `room_load.rs:202`、`F0→F1` 只有 `room_draw.rs:826` 的 `wrapping_add(1)`
   ⇒ `09` 只能在 tick 945 内被写；直接写序列观测量仍缺（§5.2 的插桩项保留为「直接观测空缺」，
   不是「结论待定」）。

### 10.2 打点 2：§8.1「可复算表」—— PASS（原文照跑，逐行一致）

```bash
cd /home/peng/workspace/metroid-decomp && python3 - <<'PY'
import glob, json
biz = {json.loads(l)["f"]: json.loads(l) for l in open("analysis/traces/bizhawk-6725M.jsonl")}
for f in range(938, 956):
    g = glob.glob(f"/tmp/rustshots-full/*_f{f:05d}_d{f}.ram.hex")
    cap = int(open(g[0]).read().strip()[0x5A * 2:0x5A * 2 + 2], 16) if g else None
    print(f, "capture(tick N-1 收口)=%s" % ("%02X" % cap if cap is not None else "-"),
          "biz=%02X" % biz[f]["room"])
PY
```

关键输出（与 §8.1 表**逐行相同**）：

```
944 capture=FF biz=09      948 capture=F1 biz=F4
945 capture=FF biz=09      949 capture=F2 biz=FF
946 capture=F1 biz=F1      950 capture=F4 biz=FF
947 capture=F1 biz=F2      951 capture=FF biz=FF
```

- glob 命中的是 `_dNNNN` 那套 capture（57 个文件，覆盖 938..955）；另有 `_capNNNN` 粗档（136 个）
  不匹配该 glob，不影响复算。`0x5A*2` 偏移读法与 `bytes[0x5A]` 等价 ✓。
  `analysis/traces/bizhawk-6725M.jsonl`（1201 行，f0..1200）存在 ✓。
- ⇒ §8.1 的读法（`capture(945)=FF` 是 tick 944 收口、`capture(946)=F1` 是 tick 945 收口、
  折叠在 tick 945）**正确**。这是本轮新增里最容易出错的一处，通过。

### 10.3 打点 3：新错误 / PASS 项是否被写坏 / 历史是否保留

| 编号 | 级别 | 位置 | 问题 | 修法 |
|---|---|---|---|---|
| A | **P1** | §8 evidence 摘要（:307-314） | **旧裁定原地残留**：「裁定：**时机错（请求晚 ~2 帧；`F` 链整体 +2 平移）**；`09` 缺失可能是同 tick 覆写（**未证**）」——与 §5「不再声称整体晚 2 帧」「已证同 tick 折叠」直接互斥；同一 bullet 还把 §3 已更正的「`EA19` inc **逐帧 +1**」写回，并把已判错的「5 种注册**全 ok**」重新当作结论 | 三行全改：①判据改「注册返回空 GUID」；②写点改「inc 速率随相位（boot 一帧多格）」；③裁定改新 §5 三成分 + 「已证」 |
| B | **P1** | §4 表头 + 946/947 行 + 更正块 | 表头写「`capture(f+1)` = tick `f` 收口」，**但列里的值是 `capture(f)`=seam(f)=tick `f-1` 收口**（f945 写 `FF`，而 `capture(946)=F1`）；946/947 标「无行（seam）」——`capture(946/947)` **存在且都是 `F1`**，等于把致命空洞又留了一遍；更正块例句「f945 capture = `F1` 应读作 tick 944 收口 = `F1`」**错位一帧**（`capture(945)=FF`；`F1` 是 `capture(946)`=tick 945 收口），与 §8.1 正确表述自相矛盾 | 表头改「`seam(f)=capture(f)` = tick `f-1` 收口」并把 946/947 补成 `F1`/`F1`（这两行正是「Rust 在 945 已到 F1」的证据行）；或整列改取 `capture(f+1)` 后逐行重排。更正块例句按 §8.1 口径重写 |
| C | P2 | §4 读数 3 | 「Rust：`F1,F2,F4,FF`（与 Biz 同形）」与 §5「Rust 在 `F1` 停 3 个 tick」互斥（post-work 轴 Rust 实为 `F1,F1,F1,F2,F4,FF`） | 标注该行是 seam 轴；补 post-work 轴编号 |
| D | P2 | §5 第三成分 / §6.3 | 「步进节律」把「首步延迟 2 tick」与「1 tick/格」混成一条（见 10.1-2） | 拆成两条，并把首步延迟挂到 §6.3 的 vblank/budget 未做项 |
| E | P3 | 标题层级 | 有两个 `## 8`（`:269` 复现命令、`:307` evidence 摘要），§7 与 §8 同名「复现命令」 | evidence 摘要改 §9 |
| F | P3 | 顶部「历史主张一律**原地**保留 + 日期更正」 | 与事实不符：旧 §5 四行裁定被整段替换（只剩 §5.2 条件 1 的删除线与顶部摘要），§4 保留了 | 或把旧 4 行以删除线留在 §5，或改述为「旧主张汇总于顶部摘要，§5.2 逐条留痕」 |
| G | P3 | §4.1 末条「`$0014` 锁存 **967/967 全同**钉死」 | 未写窗口。我复算：f40..1006 窗口内 **960 个可比帧 0 失配**（与 967 一致，状态文件缺 7 帧）；但**全段 1179 帧有 4 处不同**（f1/f3 上电相；f1133/f1134 Biz `joy1_latch=9` vs Rust `1`） | 加「窗口 f40..1006」，并注明 f1133/1134 是 Biz 侧毛刺（本报告 §9.3 已记） |
| H | P3 | §5.2 条件 2 | 「此风险已登记（见 §6）」指向的 §6 现在写「标定**已完成**」，悬空引用 | 改「已标定为逐窗口 −2..+2（§4.1），故帧级主张限定 BizHawk↔Rust 轴」 |
| I | P3 | 顶部「已由复核证实」 | 与我 §3 的证据级别不符：是「A 级边界读数 + B 级路径反推」，不是写序列直测 | 补「推断链见复核报告 §3.2/§3.4；直接观测仍缺」 |

**PASS 项没被写坏（逐条确认）**：

- §3.1「`ctwrite` ↔ `watch` 恒 **+14**」原样保留，与我 §2 三段实测一致 ✓。
- §3 的「`inc RoomNumber` 每帧只执行一次」过强表述**以日期更正块保留**（符合纪律）✓。
- §4.1 跨核标定的数值未被改动，与我独立复算一致（boot −2 / 中段 0 / 过渡 +2）✓。
- §6.2 从「未做跨核标定」改为「已完成标定，未做的是漂移原因」——上一轮的 P2 自相矛盾**已消除** ✓。
- §5.2 条件 1 以删除线 +【已证伪/已闭合】保留，历史可追溯 ✓。
- asm/RAM/Rust 锚点引用全部复核通过（`prg7_engine.asm:8673-8675/8878/8687/7703`、
  `ram_map.rs:216`、`room_load.rs:57,202`、`room_draw.rs:720,826,851`；另 `room_draw.rs:826`
  确为 `mem.set_room_number(room.wrapping_add(1))`，`rust_screenshot.rs:411` 确为
  「ram.hex = 活体（tick(N-1) 收口…work(N-1) 后/NMI(N) 前）」）✓。

### 10.4 final verdict

**pass-with-findings**。核心改判（结构差为主 + 请求 +1 帧为次 + 非平移）成立且可复算，
§8.1 这个新加强点站得住；但 **A、B 两条 P1 必须先修**——它们正好落在
「给 commit message 引用 / 给读者的一致性摘要」和「三方对照表」这两个传播位，
留着会把已撤回的「晚约 2 帧 / 整体 +2 平移」重新扩散出去。

复审回流只需给我两处 diff：§4 表（表头+946/947 行+更正块例句）与 §8 evidence 摘要三行。
