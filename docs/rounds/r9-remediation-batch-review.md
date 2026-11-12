# r9 整改批次复核档：b811aa05 / 58859f65 / bc39302f+9727e7ad / cbd0fce8 / 2fc62865 / cecffbf4 / d88acc59 / 31d51b07 —— 独立对抗复核

- **复核人**：verifier 对抗复核员（独立复核方，非实施方；证伪立场）
- **复核基点**：`31d51b07`（origin/master 已推）。⚠ 复核进行期间树上新落 2 个范围外 commit：`bfc26d6e`（scenario-lint WARN 启发式，非阻塞类）、`5c518e3e`（kb 文档）——均为增量、零 gates/`data/status` 触碰，未纳入本批判定。
- **verdict：pass**（发现的问题均为低危/潜在级，不阻塞本批入库；见 F1'…F6'）
- **Confidence: 88**

---

## A 面：asm/ROM 逐行重导（全部亲验，未照抄 commit message）

### A1 OAM 结构与槽位勘误（b811aa05）✅
- `refs/m1disasm/SRC/constants_ram.asm:3-8`：`.struct OAMSprite { y db / tileID db / attrib db / x db }`——4 字节/精灵，亲读确认。
- `constants_ram.asm:828`：`SpriteRAM instanceof OAMSprite $40 startfrom 0`（`.enum $0200`）⇒ `SpriteRAM.1`=`$0204`、`SpriteRAM.2`=`$0208`。rust 改后写 `$0204-$0207` / `$0208-$020B` ✅。
- asm 写入点亲读：`prg0_title.asm:2570-2598`（光标：`lda FrameCount/and #$08` 门 + `PasswordCursor<0x0C ? $3F : $4F` + tile `$6E` + `OAMDATA_PRIORITY` + `CursorPosXTbl`）与 `:2660-2677`（CharSelect：`CharSelectYTbl`/`$6E`/`$20`/`CharSelectXTbl`）。rust `title/mod.rs` enter_password_body 各字段逐一对应（含 `frame_count & 0x08` 门、`cursor>=0x0C` 减 `0x0C`、表兜底）✅。

### A2 0x17 冻结拍数（58859f65）✅
- `ClearAll ; 00:909F`（prg0_title.asm:2316-2328）：`ScreenOff → ClearNameTables → EraseAllSprites → … → jsr WaitNMIPass(:2327)`——显式第 2 次帧等待 ✅。
- 第 1 次等待在 `ScreenOff ; 07:C439`（prg7_engine.asm:1123）内：`lda PPUMASK_ZP / and #~(BG_ON|OBJ_ON) → WriteAndWait → WaitNMIPass_`——ScreenOff 自带帧等待 ✅。
- ⇒ 0x17 路径（LoadPasswordScreen 00:911A → ClearAll）两次等待跨 3 帧、FC 只 inc 一次；rust `title_bridge.rs:42-43` `t1f==0x17 && title_cursor<2` 冻前两段（cursor0=ScreenOff+SpinWait(1)、cursor1=ClearNameTables+EraseAllSprites+WaitNMIPass，见 title/mod.rs:1068-1085），cursor2 起 PPU 装载不冻 ✅。拍数真值以 emu 逐帧实录为准（f501-f504=237 ⇒ f505=238，commit 内实验矩阵四变体唯一自洽）✅。

### A3 EndVRAMStruct / WritePPUByte ROM 字节（2fc62865）✅（附勘误）
- bank7 固定 `$C000`，fileoff=16+7*0x4000+(cpu-0xC000)。亲验：
  - `07:C36B` → fileoff **0x1C37B** = `9d a1 07 e8 e0 4f 90 0a ae a0 07 a9`（=sta VRAMStructBuffer,x / inx / cpx #$4F / bcc @RTS / ldx VRAMStructBufferIndex / lda #$00…）✅ 与 commit 引字节序列逐字节一致；
  - `07:C376` → fileoff **0x1C386** = `a9 00 9d a1 07 68 68 60`（lda #$00 / sta VRAMStructBuffer,x / pla pla rts）✅。
  - 【勘误】任务书/描述中的 fileoff `0x1C36B/0x1C376` 偏了 0x10（未加 16 字节 NES header）；**字节序列本身完全正确**，commit 代码注释里的偏移同此偏差，属标注问题非语义问题（见 F5'）。
- `VRAMStructBuffer ds $4F`（constants_ram.asm:1182，`$07A1-$07EF`）——`cpx #$4F` 判据与 size 一致 ✅。
- asm 分支方向亲读：`cpx #$4F / bcc @RTS` ⇒ x<0x4F 继续、**x>=0x4F 溢出**（ldx 回段首→EndVRAMStruct 在段首写终止符+pla pla 早退）。rust `term_off > 0x4F` 判溢 ⇒ **==0x4F 边界不等价**（见 F1'）。

### A4 more_init ScreenOff（cecffbf4）✅
- `prg7_engine.asm:1871` 亲读：`jsr ScreenOff ;($C439)Turn off Background and visibility`，位于清环后 `ClearNameTables` 前 ✅。
- `PPUMASK_ZP db ;$FE`（constants_ram.asm:797）✅。rust 补 `read_ram(0x00FE) & !0x18 → write_ram`（BG|OBJ 两 bit）与 asm `and #~(BG_ON|OBJ_ON)` 逐位对应 ✅。

### A5 F5/F7 场景件（d88acc59/31d51b07）
- `kill_orphan_mesen`（scenario_run.py:851-905）：pgrep 两族 → /proc cmdline 含本树根或 cwd 在本树才杀；OSError 跳过 ✅（退化行为见 F3'）。
- 哨兵测试：预置 `$FE=0x06` + `render.ppumask=0x06`，断言两处均不变 + 旗不误置——**加固非削弱**（旧形预置 0x1E 时「无条件写 0x1E」的坏实现可同过）✅。

---

## B 面：防弱化审计 ✅

- 9 个 commit 逐个 `git show --stat`：触碰面 = crates/game-core（5）/ docs（4）/ tools/scenario_run.py（1）；**gates/ 与 data/status/ 零改动** ✅。（`data/status` 的基线改动来自范围外更早 commit f7199a74 链，不在本批判定内。）
- 58859f65 **未改任何测试**（仅 title_bridge.rs 注释+判据），无断言削弱；31d51b07 的测试改动是**净加固**（哨兵换 0x06 + 新增旗断言）✅。
- `unported!` 语义核查：2fc62865 的 `unported!("bank07:C36B-WritePPUByte-full")` 是确定性 fail-loud 缺口上报（bank:addr 定位），非静默回退 ✅；9727e7ad 纯格式（剥「（出现第N次）」后缀/分隔线，50/50 行成对替换，围栏 42 枚成对平衡，52 行降权标注保留）✅。
- r9-F1 否决独立重验：亲手以 `gates/cli.py` 的 `l4_ratchet_violations` 纯函数对当前 `data/status/l4_ratchet_baseline.json` × `shots_rust/matrix.json` 跑**三口径**（缺省 / for_update / for_rebase）——**均 0 违规** ✅。真相：matrix 分母 122 格 = 110 match + 2 diff + **10 flaky**（jumpledge 7 + walkfcprobe 3），flaky 格有 `matrix.flaky_cells` 多 run 观测留痕（n=4，verdicts 序列+run id+code_state），按 2026-11-03 lead 裁定的 B 径豁免锚值——**r9-F1「10 格未锚定」主张确不成立**（是 flaky 豁免，非「全部锚定」；豁免有留痕，非静默容忍）。

---

## C 面：独立复跑（全部亲手跑，主树，单实例串行——等待 lead 在跑的 run-all 排空后才起本方 run）✅

1. `cargo test --workspace`：**0 failed**（gate 侧 rust-tests 亦 PASS，540=基线 540）✅
2. `timeout 1800 python3 tools/l4_diff_run.py run case-password-justin`：**一致 12 · 失配 2 · 阻塞 0**；失配格 = f700=**23px** / f1100=**42px**——与挂账定谳值逐像素吻合，未当移植错 ✅（本次 23/42 vs 基线锚记录 71/90，恰实证「浮动滞后」机制，见 F4'）
3. `python3 gates/cli.py check`：**SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2 / CHECK: PASS**——与验收期望完全一致（trace-diff/visual-diff 均为显式 TOLERATED 账目，l4 PASS match 110/diff 2/blocked 0）✅
4. `git status` 只读核对：**树净** ✅
5. Mesen 纪律：确认 lead 的 run-all 排空、全程单实例串行，未并行 run ✅

---

## D 面：主动证伪发现（问题清单，均不阻塞）

### F1'（低-中，潜在边界不等价）：F3 修的 0x4F 边界差一
- asm：`cpx #$4F / bcc @RTS` ⇒ 溢出条件 **x>=0x4F**（x=idx+4 时即触发：ldx 回段首写终止符+pla pla 早退，**后续结构数据不再写**）。
- rust（title/mod.rs:1269-1273）：`if term_off > 0x4F { unported!(…) }` ⇒ **term_off==0x4F 时 rust 走正常路径**（终止符写 `$07A1+0x4F=$07F0`、不 unported、继续），而 asm 此时溢出路径终止符写段首。
- 缓解：单字符段 idx 小、`idx+4==0x4F` 现路径不可达（rust 注释自称 idx+4≤0x4F 恒成立——但「≤」含等于，恰是分歧点），当前无像素后果。**建议**：改 `>= 0x4F` 或在注释显式标注「==0x4F 处与 asm 溢出语义不等价」。

### F2'（低，同类残留）：密码屏 15 字节消息段仍用 `&0x7F` 取模
- title/mod.rs:1219-1223（PW_ERR/PW_NOERR msg 写入）逐字节 `0x07A1 + (x & 0x7F)`——若 idx+19 越过 0x4F，rust 静默回卷，asm 则溢出→EndVRAMStruct（终止符+早退）。与 F3 同类的静默回绕残段，当前 idx 小不可达。建议同 F1' 一并标注或加同款 fail-loud。

### F3'（低，平台退化）：F5 /proc 判断非 Linux 退化
- Unix 非 Linux（如 macOS 无 /proc）：所有 pid 在 `open("/proc/…")` OSError 时跳过 ⇒ `kill_orphan_mesen` 退化为**无操作**（宁漏杀不误杀，方向正确，孤儿清扫失效但不制造事故）。Windows 路径走 CIM+mesen2 路径过滤，未加 worktree 隔离——跨树误杀风险在 Windows 侧仍存在（本项目实际跑 Linux，低危）。

### F4'（信息，文档措辞）：findings 定谳值与基线锚值的浮动未显式写范围
- `docs/findings/oracle-present-pipeline-lag.md` 记 f700=23/f1100=42，而 `l4_ratchet_baseline.json` 锚记录 71/90；本方复跑实测恰得 23/42——机制（lag 0-4 帧浮动）成立，但文档宜补一句「像素值随 lag 浮动（观测 23-71 / 42-90），定谳的是机制非固定值」，防后人拿单一数值对不上时误判回归。

### F5'（信息，标注勘误）：commit/注释中的 ROM fileoff 偏 0x10
- 2fc62865 注释「fileoff 0x1C36B/0x1C376」应为 0x1C37B/0x1C386（漏加 16 字节 NES header）；字节序列本身亲验一致。纯标注问题，建议顺手更正（原地标注纪律）。

### F6'（信息，测试缺口）：0x17 冻结判据无专属单测
- 58859f65 的 `title_cursor<2` 判据靠 emu 逐帧实录 + L4 锚格背书（充分但为黑盒）；`title_no_return` 现有单测只钉 0x00 臂（title_bridge.rs:310）。建议补一条 t1f=0x17、cursor=0/1/2 三态的纯函数级单测防误改。

### 文档一致性核查 ✅
- `r9-darkwindow-transition-review.md`（558 行，42 围栏成对平衡）各 F 项整改对应关系：F3→2fc62865（部分，见 F1'/F2'）、F4→cecffbf4（全量，$FE 清位逐位对应）、F5→d88acc59（worktree 隔离落地）、F7→31d51b07（哨兵加固）——与代码现状一致；F2 定性（emu 截图/dump 亚帧不同步）与 findings 挂账档六步证据链互证一致。
- `oracle-present-pipeline-lag.md` 处置段（挂账不动/不容忍/不锚定/不白名单、实验补丁已还原）与 C 面实测（仍 2 diff、check 全绿、无白名单条目）一致 ✅。

---

## 结论

| 项 | 判定 |
|---|---|
| b811aa05 OAM 槽位勘误 | **pass**（A 面双侧全对齐） |
| 58859f65 FC 冻结拍数 | **pass**（asm 两等待亲读 + 实验矩阵自洽；见 F6'） |
| bc39302f+9727e7ad 复核档入库/渲染修复 | **pass**（纯格式，内容无损） |
| cbd0fce8 findings 挂账档 | **pass**（与代码/门禁现状一致；见 F4' 措辞建议） |
| 2fc62865 F3 整改 | **pass（带保留）**：==0x4F 边界与 asm 溢出语义仍差一（F1'）+ 同类残段（F2'），均现路径不可达、无像素后果 |
| cecffbf4 F4 整改 | **pass** |
| d88acc59 F5 整改 | **pass**（Linux 正确；非 Linux 退化安全，F3'） |
| 31d51b07 F7 整改 | **pass**（净加固） |
| r9-F1 否决复核 | **否决成立**（三口径 0 违规亲算；10 格=flaky 豁免且有 n=4 留痕） |

**总 verdict：pass** —— 本批 8 commit 可入库；F1'/F2'/F5' 建议作为低危后续卡（不阻塞）。
Confidence: 88（A/C 面全亲验；扣分项：f700/f1100 机制未能下潜 Mesen API 内部（继承 r9 降权）、F1' 边界为静态推演未做动态触发实验）。
