# r9 复核档：转场暗窗建模 + EndVRAMStruct 终止符 + 采集相位守卫（f7199a74 / b60974b8 / 34fc4ecd）——独立对抗复核

- **复核对象**：`git log --oneline -3`
  - `f7199a74` feat(core): 转场暗窗建模（ScreenOff/ScreenOn 各等一 NMI）
  - `b60974b8` fix(core): EndVRAMStruct 终止符补齐（密码屏单字符段）
  - `34fc4ecd` fix(tools): 采集迟到缝守卫（请求帧无 spin-exit 时不跨帧兑现）
  - 范围：`git diff 863f258a..f7199a74`（11 files changed, 149 insertions(+), 31 deletions(-)）
- **复核角色**：verifier（证伪立场，独立复跑；全部结论来自本人实跑与原文对读，不采信提交方验证段）
- **复核时间**：2026-11-12
- **复核基线**：干净 worktree `/tmp/verify-r9`（`git worktree add /tmp/verify-r9 f7199a74`，独立 `CARGO_TARGET_DIR=/tmp/verify-r9-target`），ROM 为复制而非 symlink，`sha256 = e6e6b7014685adae447ebb3833242815747bc1e5df83ade79f693fb67cf565b6`
- **Verdict：needs_revision**
  - 三处代码改动本身**未发现移植错**：asm/ROM 逐行对读成立，像素验收在干净环境下达到 `match 120 / diff 2 / blocked 0`，`cargo test` 536 passed / 0 failed，gates 无任何削弱（`gates/` 零改动、SHA256SUMS 自验通过、unported! 52→52 未降级、TDD 能真红）。
  - 但**验收口径与门禁现状和提交方叙事不匹配**：`python3 gates/cli.py check` 实测 **PASS=12 FAIL=2**（非期望的 FAIL=0），l4 门禁因**历史 10 个未锚定采集点**直接 FAIL；`trace-diff` 亦为既存红项。同时两条被写入代码注释的「根因」叙事（f700/f1100 的 71/90px = 「帧号错位 2 帧」/「CHR 低址垃圾段」）**被我方像素级证据证伪**。
- **Confidence：88**
  - 直接实测（A 级）：干净环境 4 条命令全部亲手跑、逐格数字、png/ppu.hex/ram.hex 双侧逐字节对比、门禁原文逐行。
  - 降权项（各 -2~-4）：①f700/f1100 的**机制**只在 oracle 侧产物内证到「emu 自身截图与自身 dump 不同相」，未下潜到 Mesen2 API 内部（哪个回调相位 flip surface），故只能断言「不是 rust 的 PPU 状态差」，不能断言「rust 渲染 100% 正确」；②`trace-diff` FAIL 的 samus-move3 空缓存属既存欠账，本轮只做了「父 commit 同样红」的对照，未做根治验证；③复核期间主仓库有另一个 agent 并发跑 L4 并改未提交代码（见 §F5），隔离 worktree 规避了产物竞态，但主仓库状态不可信。

---

## 问题清单（按严重度排序，均带 A 级证据指针）

### F1（高，验收口径）：`gates/cli.py check` 实测 FAIL=2，与「expect PASS=13 FAIL=0」不符；l4 因历史 10 个未锚采集点 FAIL

实跑（`/tmp/verify-r9`，pid 3758183，16:24 启动 → 16:53 落盘，14935 B）：

```
$ python3 gates/cli.py check > /tmp/check-r9.log
SUMMARY: PASS=12 FAIL=2 SKIP=1 TOLERATED=1
CHECK: FAIL —— trace-diff; l4。按 AGENTS.md §4 定位层后修复根因，禁止调阈值/加白名单掩盖。
```

l4 项原文（节选，完整 10 条被 `|` 截断）：

```
[FAIL] l4 (ENABLED, README: 启用（Task 6）) L4 逐采集点 ratchet 回退 10 处
（match 集合只增不减 + blocked 只减不增，非聚合率）:
case-jumpledge-probe/550:walk-mid: 未锚定采集点（现态 match）——新格必须同 commit 锚定
（baseline_update l4）——v2 A-lite③阻断；…690:pre-jump…；…705:takeoff…；…730:rise…；
…755:peak…；…780:land…｜用例 14 · 分母 122（blocked 0 计分母不计 pass）｜match 120 diff 2 blocked 0
```

**归因（关键，决定本条是否记在实施方头上）**：

| 检查 | 结果 |
|---|---|
| `git show 863f258a:data/status/l4_ratchet_baseline.json` cells | **110**（jumpledge=**1**、walkfc=**2**） |
| `git show 863f258a:cases/case-jumpledge-probe.txt \| grep -c "^capture"` | **8** |
| `git show 863f258a:cases/case-walkfcprobe.txt \| grep -c "^capture"` | **5** |
| `f7199a74` baseline cells | **112**（只增 2 格 1191/1197，10 缺格原样继承） |

⇒ **「用例 8/5 格 but 基线只锚 1/2 格」的错配在父 commit 就存在**，l4 FAIL 不是本轮引入，本轮也**未使其恶化**（本轮唯一新格 1191/1197 已同 commit 锚定，无 ③ 违规）。10 格在 fresh 矩阵里全部 `state=match, diff=0`（**不是回退，是机械欠账**）。

**但**：任务下达的验收期望「PASS=13 FAIL=0 SKIP=1 TOLERATED=2」与仓库现状矛盾——仓库现状是 `PASS=12 FAIL=2 SKIP=1 TOLERATED=1`。这个差异必须由 lead 裁定，不能由 verifier 用「反正不是本轮引入」抹平。

**最小整改**：
1. 一次 `baseline_update l4` 补齐 10 格（`case-jumpledge-probe` 的 550/690/705/730/755/780/800 + `case-walkfcprobe` 的 820/1020/1400 全为 match 0px，直接锚定即可），使 l4 转 PASS。**这不需要改任何阈值/白名单**，是纯补锚。
2. 后续提交的验收期望值请以实时基线为准（当前应为 122 格 / match 120 / diff 2），不要再引用已过时的 112 格 / match 110 口径。

---

### F2（高，结论定性）：f700=71px / f1100=90px 的残差被实施方定性为「渲染相位族」，机制表述与像素证据不符

实施方主张（写进 `crates/game-core/src/title/mod.rs:1258-1261` 与 `crates/game-core/src/frame/mainloop.rs` 注释、以及主仓库未提交的 `title_bridge.rs` 改动注释）：
- `title/mod.rs` 注释：「旧形缺此写 ⇒ 消费端 `vram_struct_write` 在本段（4B）之后继续按下一段解析缓冲残留 ⇒ 垃圾段写到 CHR 低址（实测 PPUADDR=$001F+ v=$FF/inc32、$0001 v=$00）⇒ f700/f1100 各 70/71B CHR 失配」；
- 未提交 `title_bridge.rs` 注释：「…不做此冻结则 f505=240（+2 恒偏，使 f700/f1100 取景偏 2 帧=71/90px 假失配：emu f700 ≡ rust f698 整图 0px、emu f1100 ≡ rust f1098 0px）」。

**我方实测（A 级，像素级）**：

用 f530（0px match 帧）反解 NES 调色板映射（四色单值无歧义：`0x0F→(0,0,0)`、`0x21→(100,176,255)`、`0x2A→(92,228,48)`、`0x30→(255,254,255)`），再以 `CHR@$1000` + NT + attr + PAL 纯 BG 渲染器逐帧重建：

```
frame                       |rustPNG - render(rustDump)|  |emuPNG - render(emuDump)|
f00320_title                            48                            48
f00420_menu                             48                            48
f00700_mid-1                             0                            71   <<<
f00950_mid-3                            71                            71
f01100_mid-4                             0                            90   <<<
f01191_dark-window                    2063                          2063
f02390_spawn-t20s                      490                           490
frames compared: 14 | emu PNG != emu dump: 7 | rust PNG != rust dump: 5
```

（BG pattern base 必为 `$1000`：用 `$0000` 时匹配 0/61440 像素，用 `$1000` 时 61440/61440。）

三条硬结论：
1. **rust 侧完全自洽**：`|rustPNG − render(rustDump)| = 0`（f700/f1100 亦然）⇒ rust 的截图与其自身 PPU 三面（CHR/NT/PAL）同相，无任何相位错位。
2. **emu 侧不自洽**：`|emuPNG − render(emuDump)| = 71 / 90`，**与 L4 报出的差值逐数字相等**，差异位置也完全重合。
3. 其余 12 帧两侧同号（48/48、71/71、2063/2063、490/490）⇒ 这不是工具伪影，是 f700/f1100 两帧特有的 emu 侧不自洽。

**差异像素的构成（决定性）**：

- f700：71px 集中在 **2 个 8×8 tile** — `tile(13,8)` bbox(104,64,111,71) 全 48px + `tile(4,19)` bbox(32,152,39,159) 23px。
- f1100：90px 集中在 **2 个 tile** — `tile(9,10)` bbox(72,80,79,87) 48px + `tile(24,23)` bbox(192,184,199,191) 42px。
- **两侧 dump 的 NT 值完全相同**（FF / 1A / FF / 3E）⇒ 名表内容一致。
- 颜色判据：`0x16 = (181,49,32)` 在密码屏 5 帧（f530/f700/f850/f950/f1100）的 BG 调色板里**不存在**（`BG=[0F,30,30,0F, 0F,2A,2A,21, 0F,31,31,0F, 0F,2A,2A,21]`），只存在于精灵调色板（`sp=$0F,$16,$2A,$27`）。`NES_PALETTE` 实体在 `crates/game-data/src/render/render_data.rs:14`，`0x16=[0xB5,0x31,0x20]`。⇒ emu PNG 里那 42/45 个红像素**只可能来自精灵层**。
- 但同一帧 OAM（`$0200-$02FF`）在 f700/f1100 **两侧 diff=0 且 64 个精灵 Y 全部 ≥0xEF（无活跃精灵）**。

⇒ **emu PNG 多出的正是一层「已经消失的精灵叠加」**：`emu.takeScreenshot()` 取到的是 post-render 表面（含该帧更早时刻的光标/候选字符精灵），而 `dumpPpu` 走 `emu.read(addr, 9)` 读 CPU 侧当时的 PPU 内存（`tools/scenario_capture.lua` 的 `dumpPpu`），二者不在同一 PPU 相位。

**「帧号错位 2 帧」假设已被排除**（`/tmp/lag.py`，对目标帧与所有可用 dump 帧两两求差）：
```
f00700: emuPNG~render(emuDump@f690)=71  @f700=71  @f720=71  @f750=67  @f780=104
f01100: emuPNG~render(emuDump@f990)=90  @f1020=90 @f1050=90 @f1080=90 @f1100=90
```
即使取最近的 f750 也只降到 67，f1100 与帧号完全无关 ⇒ **不是整帧滞后/错位**。且逻辑上也不成立：若真错 2 帧，两侧 dump 内容必然不同，但实测 **CHR/NT/PAL 逐字节 diff=0**。

**结论定性（分类＝oracle/门禁口径问题，不是移植错）**：
- f700=71px / f1100=90px 不是 rust 的 PPU/渲染状态差，而是 **emu 侧截图与 dump 亚帧不同步**；
- 由此推导，`title/mod.rs` 里「垃圾段写到 CHR 低址 ⇒ f700/f1100 各 70/71B CHR 失配」的因果叙事**不成立**（CHR 段两侧逐字节相等，diff=0）；`b60974b8` 的真实收益在别处（它把 f1190 的 ppu 残差与 530/850/950/1150/1300/1430/2390 的像素全部拉回 match，见 §C），但不是这个机制。
- ⚠ 声明边界：本轮**未下潜到 Mesen2 API 内部**（哪个回调相位 flush surface），只证「emu 自身两张产物互不一致、rust 自身两张产物一致」。故**不能**据此断言 rust 渲染 100% 正确，只能说这 71/90px 不构成移植错的证据。

**最小整改**：
1. 修正 `crates/game-core/src/title/mod.rs:1258-1261` 与主仓库未提交 `title_bridge.rs` 注释里的因果叙事（把「CHR 低址垃圾段 ⇒ 71/90px」「错位 2 帧 ⇒ 71/90px」改为「emu 截图/dump 亚帧不同步 ⇒ oracle 侧伪残差」），避免后续 agent 沿错误根因继续改动。
2. 若要根治，正确方向是 **harness 侧**：让 `takeScreenshot` 与 `dumpPpu/dumpRam` 落在同一 PPU 相位（或对截图侧做「同帧 dump 反渲染校验」自检），把该假设变成可测事实——不是继续改 rust 渲染。

---

### F3（中，契约不等价）：EndVRAMStruct 终止符用 `&0x7F` 取模，与 asm 的 `cpx #$4F` 溢出语义不等价

asm 原文：
- `refs/m1disasm/SRC/prg7_engine.asm:933-945` `WritePPUByte: ; 07:C36B`：`sta VRAMStructBuffer,x` / `inx` / `cpx #_sizeof_VRAMStructBuffer` / `bcc EndVRAMStruct@RTS` / 满则 `ldx VRAMStructBufferIndex`。
- `refs/m1disasm/SRC/prg7_engine.asm:947-955` `EndVRAMStruct: ; 07:C376`：`lda #$00` / `sta VRAMStructBuffer,x` / `pla` / `pla` / `@RTS: rts`。
- ROM 字节佐证：`EndVRAMStruct` fileoff **0x01C376** = `a9 00 9d a1 07 68 68 60`；`WritePPUByte` fileoff **0x01C36B** = `9d a1 07 e8 e0 4f 90 0a ae a0 07 a9`。
- 缓冲尺寸：`refs/m1disasm/SRC/constants_ram.asm:1168` `VRAMStructBufferIndex db ;$07A0 … #$4F bytes max`；`:1182` `VRAMStructBuffer ds $4F ;$07A1-$07EF`。

⇒ asm 语义：终止符写在**当前 X**。正常路径 = 最后写入字节之后一槽；**溢出路径（X 被重置为 `VRAMStructBufferIndex`）= 写回段首，覆盖段首字节**。

rust（`crates/game-core/src/title/mod.rs:1262-1263`）：
```rust
e.push((0x07A0, ((idx as u16 + 4) & 0xFF) as u8));
e.push((0x07A1 + (((idx as u16 + 4) & 0x7F)), 0x00)); // EndVRAMStruct 终止符
```
用 `&0x7F`（模 128），asm 是 `cmp #$4F`（模 79 且溢出回卷到段首）。

**现状影响**：本路径不可达。密码屏每字符只写 4 字节、`idx ∈ {0,4,8,…}`、一轮最多 8 字符 ×4B = 32B ⇒ `idx+4 ≤ 0x4F` 从未越界，实测所有用例 0px。⇒ **当前无像素后果**。

**但是**：若将来 `idx` 接近 `0x4F`（例如长消息段复用同一缓冲），asm 会把终止符写到**段首**，rust 会写到 `(idx+4)&0x7F`，对 `idx=0x4C` 得 `0x50` ⇒ 实际地址 `$07A1+0x50 = $07F1`，**越出 `$07A1-$07EF` 缓冲尾**（写进 `$07F0+` 区）。这是**潜在越界写**，且契约与 asm 不等价。

同类偏差：`crates/game-core/src/title/mod.rs:1414-1419` `vram_struct_buffer_events(idx, blob)` 亦用 `(idx + i) & 0x7F`。

**最小整改**（低风险，不需要改任何阈值）：
```rust
let end_idx = idx as u16 + 4;
let term_addr = if end_idx >= 0x4F { 0x07A1 + idx as u16 } else { 0x07A1 + end_idx };
e.push((term_addr, 0x00));
```
并加一条单元测试钉住 `idx=0x4C` 的溢出路径（期望写 `$07A1+0x4C`，而非 `$07F1`）。若判定「本路径永久不可达、不修」，至少在该行注释里显式标注「与 asm 溢出行为不等价，本路径 idx+4 ≤ 0x4F 保证不触发」，避免后续误当成等价实现复用。

---

### F4（低-中，asm 建模缺口）：`more_init` 未建模 `jsr ScreenOff` 对影子 `$FE` 的清位

asm `MoreInit: ; 07:C81D` 内 `refs/m1disasm/SRC/prg7_engine.asm:1871` 明确有 `jsr ScreenOff`，而 `ScreenOff: ; 07:C439`（`refs/m1disasm/SRC/prg7_engine.asm:1123-1127`）语义是 `lda PPUMASK_ZP / and #~(BG_ON|OBJ_ON)`（`and #$E7`）+ fallthrough 到 `WriteAndWait` 写影子（ROM `0x01C439` = `a5 fe 29 e7 85 fe 20 34 c4 …`）。

rust `crates/game-core/src/frame/mainloop.rs:2958 more_init` 内**无任何 `0xE7` / `!0x18` 掩码写**（`grep "0xE7\|!0x18\|!(0x08" crates/game-core/src/frame/mainloop.rs` 仅命中 `:895` 无关阈逻辑与 `:3180` 数据表常量）；ScreenOff 只以 `NMI_STATUS=0`（`:2973`）+ `iter_wait()`（`:2974`）表达「等待」。暗窗期 `$FE` 从 `0x1E → 0x06` 的实际来源是 `crates/game-core/src/frame/title_bridge.rs:492-493` 的 `& !0x18`（InitializeGame 的 ScreenNmiOff），不是 more_init。

**实跑后果**：无（三帧 `$FE` 与 emu 完全对得上）：
```
f01150_pw-done       FE emu=1E rust=1E | FF 90/90 | MR 00/00
f01190_submit        FE emu=1E rust=06 | FF 90/90 | MR 00/01
f01191_dark-window   FE emu=06 rust=06 | FF 90/90 | MR 01/02
f01197_last-dark     FE emu=06 rust=06 | FF 90/90 | MR 01/02
f01300_post-submit   FE emu=1E rust=1E | FF 90/90 | MR 08/08
```
⇒ 暗窗边界帧 `$FE` 双侧一致；唯一差是 `MainRoutine` 相位 rust 领先 1（f1190 0/1、f1191 与 f1197 1/2），但三格像素全 0px ⇒ **不影响验收**，仅记录。

**最小整改**：在 `more_init` 的 ScreenOff 处补 `write_ram(0x00FE, read_ram(0x00FE) & !0x18)`（并确保不破坏 `more_init_declares_frame_gates` 的 `vec![1,0,0,1]` 断言），使 asm 契约面闭合；或显式注释「ScreenOff 影子清位在本模块由 title_bridge ScreenNmiOff 代偿，more_init 内不重复」。**不阻塞本轮 verdict**。

---

### F5（低，纪律/流程）：复核期间主仓库被并发 agent 改写，且改的是「未验证的根因假设」

- `git status --porcelain`（主仓库）在复核过程中出现：**` M crates/game-core/src/frame/title_bridge.rs`**（至今未提交），diff 内容 = `main_title_routine` 里 `mem.title_no_return = matches!(..., 0x00|0x01|0x15|0x1B)` → 加入 `0x17`，并附「【FC 相位 +2 根修】…emu f700 ≡ rust f698 整图 0px、emu f1100 ≡ rust f1098 0px」的长注释。
- 并发进程实录（16:58）：`3830636 python3 tools/l4_diff_run.py run case-password-justin` → `3830688 scenario_run.py run …/case-password-justin.txt` → `3830691 Xvfb :109` → `3830692 …/tools/mesen2/Mesen …`。主仓库 `shots/`/`shots_rust/` 像素会在复核期间被覆盖（实测 `shots/case-password-justin_f01100_mid-4.png` mtime 16:12:53 晚于我的读取时刻）。
- 另：`tools/scenario_run.py:846-880 kill_orphan_mesen()` 用 `pgrep -f "tools/mesen2/Mesen"` **无差别 SIGKILL**，两个 worktree 的 Mesen 会互相杀掉 ⇒ 我在主仓库早期看到的一次 `blocked 4 / flaky 10` 是**并发互杀噪声**（`case-collide-probe` 的 emu 腿在第 3 个采集点后被外部杀掉：lua.log 只有 3 行 SHOT-SEAM、`outputs.shots` 只有 3 张、`trace_lines 692/900 frames`），**不是被测代码回退**。我用隔离 worktree 串行复跑规避了这一点（干净跑得 `blocked 0 / flaky 0`）。
- 该未提交改动主张的「帧号错位 2 帧」根因，已被 §F2 逐条证伪。

**最小整改**：并发核查请在独立 worktree 进行（`git worktree add`），勿在主仓库跑 L4 并留未提交改动；`kill_orphan_mesen` 的 `pgrep -f` 模式建议加 worktree/端口过滤，否则同机多 worktree 复核必互相清场。

---

### F6（低，流程）：anchor history 的 `evidence` 字段为空串（历史惯例，非本轮新引入）

`data/status/l4_ratchet_baseline.json` 的 history 末条：
```json
{"ts":"2026-11-12T07:41:21+00:00","kind":"anchor","evidence":"","reason":"转场暗窗建模（ScreenOff/ScreenOn 各等一 NMI）+ 用例新增两格相位锚（f1191 dark-window / f1197 last-dark，均 0px）——新格同 commit 锚定，scenario sha 随用例更新；match 108→110 / cells 110→112","match":110,"diff":2,"blocked":0,"cells_total":112}
```
21 条 history 中最近 5 条 anchor 有 4 条 `evidence=""`（2026-11-05 / 11-06 / 11-11 亦然），唯一带证据的是 2026-11-09 的 rebase 条 ⇒ **历史惯例，不是本轮引入的假陈述**。`reason` 文本数字与实跑一致（match 110 / cells 112 的**锚定前**口径；注意该口径在下一次补锚 F1 的 10 格后会变成 120/122）。

**最小整改**：后续 anchor 写 `evidence`（至少给 `shots_rust/matrix.json` 的 mtime+sha、asm 地址、逐格 0px 摘要）。

---

## A 面：asm/ROM 逐行对读

唯一真相源 `refs/m1disasm/SRC`，全部逐行读完（非抽样），并按 ROM 字节交叉验证。

### A1 ScreenOff / WriteAndWait / WaitNMIPass_ / ScreenOn / WritePPUCtrl / ScreenNmiOff

| 标号 | asm 位置 | 语义 | ROM 字节（fileoff） |
|---|---|---|---|
| `WaitNMIPass ;07:C42C` | `prg7_engine.asm:1108-1115` | `jsr ClearNMIStat` + `@loop: lda NMIStatus / beq @loop` | — |
| `ClearNMIStat ;07:C434` | `prg7_engine.asm:1117-1121` | `lda #$00 / sta NMIStatus / rts` | — |
| `ScreenOff ;07:C439` | `prg7_engine.asm:1123-1127` | `lda PPUMASK_ZP / and #~(BG_ON\|OBJ_ON)`（`#$E7`）**fallthrough** | `0x01C439`: `a5 fe 29 e7 85 fe 20 34 c4 a5 1a f0 fc 60` |
| `WriteAndWait ;07:C43D` | `prg7_engine.asm:1129-1131` | `sta PPUMASK_ZP`（**只写影子**） | 接续 `0x01C43D`: `85 fe` |
| `WaitNMIPass_ ;07:C43F` | `prg7_engine.asm:1133-1140` | `jsr ClearNMIStat` + `lda NMIStatus / beq` | 接续 `0x01C43F`: `20 34 c4 a5 1a f0 fc 60` |
| `ScreenOn ;07:C447` | `prg7_engine.asm:1142-1146` | `lda PPUMASK_ZP / ora #$1E / bne WriteAndWait` | `0x01C447`: `a5 fe 09 1e d0 f0` |
| `WritePPUCtrl ;07:C44D` | `prg7_engine.asm:1150-1161` | `lda PPUCTRL_ZP/sta PPUCTRL` + `lda PPUMASK_ZP/sta PPUMASK` + `lda MirrorCntrl/jsr PrepPPUMirror` | `0x01C44D`: `a5 ff 8d 00 20 a5 fe 8d 01 20 a5 fa 20 d9 c4 60` |
| `ScreenNmiOff ;07:C45D` | `prg7_engine.asm:1165-1177` | `lda PPUMASK_ZP/and #~$18/jsr WriteAndWait` → `lda PPUCTRL_ZP/and #~$80/sta $FF/sta $2000/rts` | `0x01C45D`: `a5 fe 29 e7 20 3d c4 a5 ff 29 7f 85 ff 8d 00 20 60` |

**语义判定（三问逐条）**：
1. **「影子写 vs 真寄存器落点」**：`ScreenOff`/`ScreenOn`/`ScreenNmiOff` 三者都只经 `WriteAndWait` 写 `PPUMASK_ZP`（影子 `$FE`），**真 `$2001` 只由 NMI 内的 `WritePPUCtrl`（`07:C44D`）落**。ROM 中 `ScreenNmiOff` 的 `$2000` 真写在此（`8d 00 20`），但**没有 `$2001` 真写**（对照 `0x01C45D` 字节串，`8d 01 20` 不出现）⇒ 实施方此点完全正确。
2. **「哪一帧点亮」**：`ScreenOn` → `WriteAndWait` 写影子 → fallthrough `WaitNMIPass_` **等一个 NMI** → 该 NMI 里 `WritePPUCtrl` 才 `sta $2001` ⇒ 点亮在**下一帧**，本帧仍是暗的。实施方「rust 旧形同 tick 内收完 ⇒ 无暗窗」的诊断与「修 = 延到帧门排空点」的建模方向**与 asm 一致**。
3. **「终止符位置」**：见 §F3 —— 方向正确（补 `$00`），但取模方式与 asm 溢出语义不等价。

### A2 MoreInit 尾部与其中的 jsr ScreenOff

`refs/m1disasm/SRC/prg7_engine.asm:1843-1930` `MoreInit: ; 07:C81D` 全文已读：
- 四项清、`$7A-$DE` / `$0300-$03FE` 清环（`:1859-1869`，含两条 `off-by-one bug` 注释）；
- **`:1871` `jsr ScreenOff`**（= `and #$E7` + 写影子 + 等一 NMI）；
- ClearNameTables / EraseAllSprites / DestroyEnemies / ScrollBlock 族清；
- `ldx #$04` 4 页 `lda ($00),y / sta PPUDATA`（`:1916-1923`）；
- `stx DoorPaletteChangeDir`（`:1925`）、`inx / stx PaletteDataPending / stx SpareMem30`（`:1926-1928`）；
- `inc MainRoutine`（`:1929`）；
- **`:1930` `jmp ScreenOn`**。

ROM 佐证：全 bank7 段扫 `4c 47 c4`（`jmp ScreenOn`）**唯一命中 `0x00C8AD`**；`20 47 c4`（`jsr ScreenOn`）在 `$C800-$C960` 段内 0 命中 ⇒ MoreInit 尾部确实是 `jmp`（尾部调用，不返回），与 asm listing 一致。

`InitBank2: ; 07:C583`（`prg7_engine.asm:1441-1483`）多处 `jsr ScreenNmiOff`（另 `:1381` / `:1402` 亦有）——与 titles/转场族的关屏路径一致，未见与本轮改动冲突。

### A3 密码屏链：EnterPassword / PrepareWriteVRAMStringRAM / WriteVRAMString / WritePPUByte / EndVRAMStruct

| 标号 | asm 位置 | 关键行 |
|---|---|---|
| `EnterPassword ;00:9147` | `prg0_title.asm:2417-2443` | `jsr EraseAllSprites` / START 门 `jmp CheckPassword` / `ldx #$01 stx PPUDataPending` / `ldx VRAMStructBufferIndex` / `lda #>$21A8 … #<$21A8 … #$0F` 三字节 + `jsr WritePPUByte` |
| `LoadRowAndColumn ;00:91BF` | `prg0_title.asm:2513-2540` | `lda #$11 / sta VRAMStringRAMDimensions` + `ldx $06 / ldy #$21 / jsr PrepareWriteVRAMStringRAM` |
| `InitializeGame ;00:92D4` | `prg0_title.asm:2699-2754` | 尾 `lda #_id_MoreInit.b / sta MainRoutine`（`:2738-2739`）+ **`jsr ScreenNmiOff`（`:2741`）** + `jsr LoadSamusGFX`（`:2743`）+ `jsr NMIOn`（`:2745`） |
| `DisplayPassword ;00:9359` | `prg0_title.asm:2790-2817` | 尾 `jmp ScreenOn`（`:2813`） |
| `PrepareWriteVRAMStringRAM ;00:9450` | `prg0_title.asm:2975-2992` | `stx $00 / sty $01 / ldx #<$… / ldy #>$… / stx $02 / sty $03 / jmp WriteVRAMString` |
| `WriteVRAMString ;07:C328` | `prg7_engine.asm:886-931` | 尾 `jsr EndVRAMStruct`（:931，注释「equivalent to jump, bc stack is popped by routine」） |
| `WritePPUByte ;07:C36B` | `prg7_engine.asm:933-945` | `sta VRAMStructBuffer,x / inx / cpx #_sizeof_VRAMStructBuffer / bcc / ldx VRAMStructBufferIndex` |
| `EndVRAMStruct ;07:C376` | `prg7_engine.asm:947-955` | `lda #$00 / sta VRAMStructBuffer,x / pla / pla / rts` |

ROM 字节佐证：
- `PrepareWriteVRAMStringRAM` fileoff **0x001450** = `86 00 84 01 a2 80 a0 07 86 02 84 03 4c 28 c3`（`VRAMStringRAMDimensions=$0780`）。
- `WriteVRAMString` fileoff **0x01C328** 尾部含 `20 76 c3` = `jsr $C376` ✔。
- `WritePPUByte` fileoff **0x01C36B** = `9d a1 07 e8 e0 4f 90 0a ae a0 07 a9`（`cpx #$4F` 明文可见）✔。
- `EndVRAMStruct` fileoff **0x01C376** = `a9 00 9d a1 07 68 68 60` ✔。

### A4 「恰好 7 帧」是否被硬编码成常量（过度拟合检查）

`grep -rn "1191\|1197\|1198" crates/ --include=*.rs` 的全部命中：
```
crates/game-core/src/render/mod.rs:129
crates/game-core/src/render/mod.rs:635
crates/game-core/src/frame/title_bridge.rs:497
crates/game-core/src/frame/mainloop.rs:3030
crates/game-core/src/frame/mainloop.rs:3051
crates/game-core/src/state.rs:42
```
**全部是注释**，无任何常量/条件依赖具体帧号。

暗窗长度由门队列自然导出：`more_init` 内 `iter_wait()`（`:2974`，ScreenOff 等 1 NMI）+ `iter_bogged(2)`（`:3004`，房间装载）+ `deferred_screen_on = true`（`:3033`）+ `iter_wait()`（`:3036`，ScreenOn 等 1 NMI）⇒ 门队列 `[1, 0, 0, 1]`，与 `mainloop.rs:3614-3620` 既有测试 `more_init_declares_frame_gates` 的断言 `vec![1, 0, 0, 1]` 一致。`end_of_tick_gate` 每 tick 弹一个，队列清空的下一 tick 才走空分支兑现 ⇒ **暗窗跨 f1191–f1197 是 asm 两段等待的自然结果，不是拟合常数**。实施方此点主张**成立**。

（`iter_wait()` 语义确认：`crates/game-core/src/frame/memory.rs:151` `pub fn iter_wait(&mut self) { self.iter_gates.push(1); }` —— 无参数，一次推 1 个门；`memory.rs:158` `iter_bogged(n)` push n 个 0。）

---

## B 面：防弱化审计

### B1 `gates/` 零改动

```
$ git diff 863f258a..f7199a74 -- gates/ | wc -l
0
```

### B2 `SHA256SUMS` 自验通过，ROM 未动

```
$ git diff 863f258a..f7199a74 --stat -- data/status/baselines/SHA256SUMS
 data/status/baselines/SHA256SUMS | 4 ++--
$ cd /home/peng/workspace/metroid-decomp && sha256sum -c data/status/baselines/SHA256SUMS
rom/Metroid (USA).nes: 成功
data/status/baselines/baseline.json: 成功
data/status/baselines/baseline_history.jsonl: 成功
```
新值：`baseline.json` = `b73fd93042194d4d99d4e2a1aad912ca80954e2409675e0ee01e1c8f799d59fd`；`baseline_history.jsonl` = `1e28413ced267960a9ebdcbc3e54662db54a9280c74321d2093a54d2e1e6bd08`。ROM sha 仍 `e6e6b7014685adae…`。

### B3 `l4_ratchet_baseline.json` 无「diff 格锚成 match」作弊、无删格

`case-password-justin` 逐格（`git show <rev>:data/status/l4_ratchet_baseline.json`）：

| 格 | 863f258a | f7199a74 | 独立复跑实测 |
|---|---|---|---|
| 320:title | match 0 | match 0 | 0px ✔ |
| 420:menu | match 0 | match 0 | 0px ✔ |
| 530:pwd-screen | diff 1294 | **match 0** | **0px ✔ 真改善** |
| 700:mid-1 | diff 1365 | diff 71 | 71px ✔ |
| 850:mid-2 | diff 1294 | **match 0** | **0px ✔** |
| 950:mid-3 | diff 1275 | **match 0** | **0px ✔** |
| 1100:mid-4 | diff 1384 | diff 90 | 90px ✔ |
| 1150:pw-done | diff 1294 | **match 0** | **0px ✔** |
| 1190:submit | diff 769 | **match 0** | **0px ✔** |
| 1191:dark-window | —（无格） | **新增 match 0** | **0px ✔** |
| 1197:last-dark | —（无格） | **新增 match 0** | **0px ✔** |
| 1300:post-submit | diff 14272 | **match 0** | **0px ✔** |
| 1430:spawn-t4s | diff 14272 | **match 0** | **0px ✔** |
| 2390:spawn-t20s | diff 14434 | **match 0** | **0px ✔** |

- cells 110 → 112，**只增不减**；`case-jumpledge-probe` 1→1、`case-walkfcprobe` 2→2（见 F1，缺格是继承的）。
- 8 个 diff→match 全部在独立复跑中**实测为 0px** ⇒ 真改善，不是改数。
- 2 个保留 diff（700 / 1100）诚实保留。
- 2 个新格 1191/1197 同 commit 锚定，满足 `gates/cli.py:1104` 规则③。
- `scenario_sha256` 从 `b2146fc8…` → `67c0db95…`（因用例新增两格，合理）；其余 13 用例 `scenario_sha256` 与 fresh 矩阵逐字相同（`sha_same=True` ×14，其中含本轮变化的 password-justin 按新用例重算）。
- `rom_sha256` 锚未变。

### B4 `baseline_history.jsonl` 末条 reason 无假陈述

```json
{"ts":"2026-11-12T07:41:24+00:00","kind":"metric","metric":"rust_test_count","old":538,"new":540,
 "reason":"转场暗窗建模新增 2 条 TDD（tests_deferred_screen_on）——538→540"}
```
静态 `#[test]` 计数 540（`grep -rn "#\[test\]" crates/ --include=*.rs | wc -l` = **540**；game-core 508 / game-data 28 / game-platform 4），`gates/cli.py:530-537 current_metrics()` 正是统计该子串次数 ⇒ 数字与机械口径一致。

### B5 新 TDD 能真红（实测 1 次）

在 `/tmp/verify-r9` 把守卫 `if !mem.deferred_screen_on { return; }` 临时替换为 `if true { return; }`（备份 `/tmp/mainloop.bak`）：

```
$ CARGO_TARGET_DIR=/tmp/verify-r9-target cargo test -p game-core --lib tests_deferred_screen_on
deferred_screen_on_inert_without_flag ... ok
deferred_screen_on_applies_at_drain_point ... FAILED
  panicked at mainloop.rs:678:9:
  assertion `left == right` failed: 排空点应把 ScreenOn 影子落 $FE
    left: 6
   right: 30
test result: FAILED. 1 passed; 1 failed; 482 filtered out
```
恢复后：
```
$ cp /tmp/mainloop.bak crates/game-core/src/frame/mainloop.rs && cargo test -p game-core --lib tests_deferred_screen_on
test result: ok. 2 passed; 0 failed; 482 filtered out
```

⇒ **`applies_at_drain_point` 能真红**；但 **`inert_without_flag` 在实现被完全禁用时仍绿** ⇒ 该条是弱断言（只测「没旗时不变」，禁实现时天然满足），**无诊断力**。

**最小整改（低）**：`inert_without_flag` 改成钉住「`apply_deferred_screen_on` 被调用前后 `$FE` 与 `render.ppumask` 都不变」，并在调用前显式设置一个非默认 `$FE`/`ppumask`（当前断言 `ppumask==0x1E` 依赖 `FrameMemory::default()` 的初值，初值一变断言含义就漂移），并加 `write_io` 计数断言（调用前后 `$2001` 写次数不变）。

### B6 `unported!` 未被静默降级

```
$ git grep -o "unported!" 863f258a -- crates | wc -l
52 （出现第1次）
$ git grep -o "unported!" f7199a74 -- crates | wc -l
52 （出现第2次）
$ git grep -o "unported!" -- crates | wc -l
52 （出现第3次）
$ git diff 863f258a..f7199a74 -- crates/ | grep -c "unported"
0
```
宏本体 `crates/game-core/src/gap.rs:27-33`（`record($loc)` + `panic!("unported @ {}", $loc)`）未改 ⇒ fail-loud 语义保持。

---

## C 面：独立复跑（全部亲手跑，干净 worktree `/tmp/verify-r9` @ `f7199a74`）

### C1 `cargo test --workspace`

```
$ CARGO_TARGET_DIR=/tmp/verify-r9-target cargo test --workspace 2>&1 | grep -E "^test result" | awk '{p+=$4; f+=$6} END {print "TOTAL passed="p" failed="f}'
TOTAL passed=536 failed=0
```
✔ 与期望 536 passed / 0 failed 一致。静态 `#[test]` = 540（gates 口径）✔。

### C2 `python3 tools/l4_diff_run.py run case-password-justin`

```
锚: scenario=67c0db95e4b572f8… rom=e6e6b7014685adae… sched=d4b8cdefe466e3a8…  产物 28 件
emu 腿: frames: 2600  trace_lines: 2600  captures: 99  shots: 99  wall: 54.7s
320 title 0 | 420 menu 0 | 530 pwd-screen 0 | 700 mid-1 71 | 850 mid-2 0 | 950 mid-3 0
1100 mid-4 90 | 1150 pw-done 0 | 1190 submit 0 | 1191 dark-window 0 | 1197 last-dark 0
1300 post-submit 0 | 1430 spawn-t4s 0 | 2390 spawn-t20s 0
一致 12 · 失配 2 · 缺口阻塞 0 · 错误 0（共 14）
```
✔ 与期望逐项吻合（12 一致 / 2 失配 / 0 blocked；f1190、f1191、f1197 均 0px）。

（`python3 tools/l4_diff_run.py diff case-password-justin` 实测 **EXIT=1** —— 有失配即 1，符合工具契约 exit 0=全 match / 1=有 diff/blocked / 2=fail-loud。⚠ 注意若用 `| tail -N` 管道，`$?` 会取到 tail 的退出码，不能据此判断。）

### C3 `python3 tools/l4_diff_run.py run-all`

干净 worktree fresh 矩阵（`shots_rust/matrix.json`）：
```
{'cases_total': 14, 'cells_total': 122, 'match': 120, 'diff': 2, 'blocked': 0, 'flaky': 0,
 'pass': 120, 'pass_rate': '120/122', 'verdict': 'not-pass'}
非 match: case-password-justin 700:mid-1 diff=71 / 1100:mid-4 diff=90（仅此 2 格）
```
- ⚠ **任务期望「14 用例 / 112 格 / match 110 · diff 2 · blocked 0」已过时**：真实分母是 **122 格**（= 基线 112 格 + 历史 10 个未锚格，见 F1），match 120。这与 `gates check` 内部 run-all 的实文一致（`用例 14 · 分母 122（blocked 0 计分母不计 pass）｜match 120 diff 2 blocked 0`）。
- ⚠ 并发互杀会让 blocked 虚高：我在主仓库早期见到 `cells_total 112 / match 57 / blocked 55 / flaky 10`，其中 `case-collide-probe` 的 4 格 `blocked`（`reason="缺模拟器截图"`）是 `kill_orphan_mesen()` 的 `pgrep -f "tools/mesen2/Mesen"` 把**另一个 worktree 的 Mesen** 杀掉的产品（该次 emu 腿 lua.log 只有 3 行 SHOT-SEAM、`outputs.shots` 只有 3 张、`trace_lines 692 / 900 frames`）。干净串行跑后 `blocked 0 / flaky 0` ⇒ **不是回退**。
- `flaky_cells` 在当前干净跑中为 `[]`（空 list）；主仓库那次噪声跑里是 10 个 dict 项（每项含 case/frame/tag）。核对时按 dict 列表处理。

### C4 `python3 gates/cli.py check`

见 §F1：**`SUMMARY: PASS=12 FAIL=2 SKIP=1 TOLERATED=1`** / `CHECK: FAIL —— trace-diff; l4`。期望 `PASS=13 FAIL=0 SKIP=1 TOLERATED=2` ⇒ **不符**。

16 项逐项：rom-integrity PASS / seed-integrity PASS（`reference/src/` 轨道 trace 比对 SKIP）/ assets-manifest PASS / naming-ledger PASS（WARN：738 条中第 632-738 行缺 confidence）/ knowledge-format PASS / docs-links PASS / **rust-tests PASS（「测试数 540（基线 540）」）** / **trace-diff FAIL** / **visual-diff TOLERATED** / **l4 FAIL** / completeness-ratchet PASS（57/57）/ **proof-checks SKIP**（变更面不含 S_intx）/ tree-coherence PASS / doc-sync PASS / shared-inode-locks PASS / arch-contract PASS。ratchet 7 metric 全 OK；`baseline-integrity: … sha 链与状态锚点一致`。

**trace-diff FAIL 归因**（与本轮无关，但确实是既存红项）：
```
[FAIL] trace-diff … [regen] 缓存缺失——经 tools/scenario_run.py 重采集 sc-first-room-full（响亮再生）
[FAIL] l3-sc-first-room-full: oracle trace sha256 不符 manifest（确定性破裂/缓存陈旧/篡改）
L3 帧级裁定: 26 条用例, 7 失败, 0 记账容忍｜l4 现状基线账外 7/7 项…
```
- worktree `/tmp/verify-r9` 的 `analysis/` 是**新建空目录**（`ls analysis/dump | wc -l` = 0），check 现场重采得 3178 行 / sha `7ae76fd0…`，manifest 锚 `dc651561…` ⇒ 不符。
- 对照**主仓库**（有完整 analysis 缓存）`python3 tools/diff_traces.py`：`[PASS] l3-sc-first-room-full: oracle 腿已验（sha256 对锚、3177 帧、逐帧字段合法）`；26 用例中 **0 条 sha 不符**；唯一 FAIL 是 `[FAIL] l3-samus-move3-oracle-determinism: 行数 0 != legs.oracle.rows 1383`（`analysis/traces/samus-move3.frames.jsonl` 是 0 字节、mtime 2026-10-26，manifest 锚 `e3b0c44298fc…b855` = 空文件 sha）。
- `git show 863f258a:oracle/traces/l3_manifest.json` 的 samus-move3 锚与现在完全相同 ⇒ **父 commit 就红**。
- `git diff --name-only 863f258a..f7199a74 -- oracle/` **为空** ⇒ 三方 commit 未改 oracle 场景/manifest。

⇒ **结论**：`trace-diff` 与 `l4` 两红都不是三方 commit 引入，但「check 全绿」这一验收前提在当前仓库**不成立**。这需要 lead 明确：要么先清两项欠账（F1 补锚 10 格 + 恢复 samus-move3 缓存/更新 manifest），要么调整验收期望为「关闭旧欠账后转 PASS」。

### C5 逐格对照基线：match→非 match 回归 = 0

只比较 baseline∩fresh 的 key：
- `case-password-justin`：8 个 diff→match 全部成立（0px），700/1100 保持 diff 71/90 ⇒ **无 match 缩水**。
- 其余 13 用例：baseline 已锚格全部仍 match，**无一条 match→非 match**；`stale=0`（baseline 无失效格）。
- 并发噪声跑中唯一的 4 条「match→blocked」是 `case-collide-probe` 700/800/890/900，已由 §C3 证据判定为互杀产品（干净跑全 match）。
- 用门禁纯函数对**干净矩阵**复算：`l4_ratchet_violations(baseline, matrix)` 返回 **10 条**，全部是规则③「未锚定采集点」（jumpledge 7 + walkfcprobe 3），**无规则①match 缩水、无规则②新增 blocked、无规则④scenario sha 漂移、无 rom 锚漂移**。

### C6 迟到缝守卫的真实行为（`analysis/traces/case-password-justin.lua.log`）

```
$ grep -o "reason=[a-z-]*" analysis/traces/case-password-justin.lua.log | sort | uniq -c
      3 reason=late-seam-fallback
     96 reason=spin-exit-seam
$ grep "late-seam-fallback" analysis/traces/case-password-justin.lua.log
SHOT-SEAM f=1190 at=1191 tag=submit attempts=2 reason=late-seam-fallback
SHOT-SEAM f=1191 at=1192 tag=dark-window attempts=2 reason=late-seam-fallback
SHOT-SEAM f=1197 at=1198 tag=last-dark attempts=2 reason=late-seam-fallback
```
96 次 `spin-exit-seam` **全部 `at==f`**（守卫 `pendingShot.frame == frame` 不放行任何跨帧兑现）；**无** `SHOT-SEAM-MISS` / `SHOT-UNSTABLE` / `SHOT-FAIL`。日志恒带 `at=%d`（`fulfillShot` 内 `string.format("SHOT-SEAM f=%d at=%d tag=%s attempts=%d reason=%s", ps.frame, frame, …)`）⇒ 实施方主张③成立。

---

## D 面：新问题扫描

### D1 迟到缝守卫是否引入新的漏兑现

- **相邻两帧连续请求**：`requestShot(tag)`（`tools/scenario_capture.lua:243`）登记 `pendingShot = {path, frame = frame, tag}`；若已有 `pendingShot`，先 `log("SHOT-SEAM-MISS …")` 再 `fulfillShot("seam-miss-fallback")` **兑现旧请求**，然后登记新请求 ⇒ 不会静默丢请求（旧请求会用当前帧表面兑现，日志留痕）。
- **同一帧多请求**：同上，第二条会先兑现第一条（`seam-miss-fallback`）⇒ 不丢，但旧请求的「表面相位」不是其请求帧。本用例 99 次兑现中 0 次 `seam-miss-fallback` ⇒ 该路径未被触发，**无实测证据**，属未覆盖分支。
- **捕获点极密集**：`endFrame` 内兜底（`:1220-1222`）位于 `applyFixtures()` 之后、`sampleTrace()` 之前 ⇒ **每帧首先处理迟到请求**，保证不会悬挂到下一采集点。实测 99 次兑现全部在 `at ∈ {f, f+1}` ⇒ 无悬挂。
- **结论**：本用例未观测到漏兑现。**残余风险（低）**：`seam-miss-fallback` 与「同帧双请求」路径无实测覆盖，若将来用例出现密集采集点需重验。

### D2 `late-seam-fallback` 是否可能掩盖真实差

**可能，但本用例不成立，且有一个未受保护的口径假设。**

- 机制：兜底在「请求帧 +1 的 endFrame 界」用 `takeStableShot()` 取图（`tools/scenario_capture.lua:160-175`：连续两次 `emu.takeScreenshot()` 字节相同才 stable，上限 `SHOT_STABLE_MAX=8`）。注释声称「端帧界 takeScreenshot 的表面滞后一帧 ⇒ 恰为请求帧内容」——这是**依赖 Mesen 截图滞后一帧**的实现细节假设。
- **本用例的独立验证**：f1190 / f1191 / f1197 三格实测 0px，且这三帧的 **PNG 与双侧 dump 的一致性**均通过（f1190 = 密码屏 4 色逐像素 0 差；f1191/f1197 = 双侧 1 色 61440 全黑）⇒ 兜底取到的确实是请求帧内容。
- **风险定性**：该补偿**无断言保护**。若 Mesen 版本/后端变化导致截图滞后变成 0 或 2 帧，兜底会**静默**取到错误相位，而 L4 只会表现为 diff（看起来像移植错）。⇒ 这与 §F2 揭示的 emu 截图/dump 不同相是**同一个根问题**。
- **最小整改**：加 harness 自检 —— 在任一采集点同时取截图与 dump，用「dump 反渲染」校验截图相位一致（我方 `/tmp/cmp2.py` 的方法即可产品化），把「差一帧补偿」从注释假设变成可测事实。

### D3 `deferred_screen_on` 的边界

| 场景 | 分析 | 结论 |
|---|---|---|
| 一个 tick 内多次 ScreenOn/ScreenOff | 旗是 **bool 不是计数**（`crates/game-core/src/state.rs`），多次置位等价一次 | **不构成 bug**：asm 的 `ScreenOn` 语义是「把影子 OR 上 `$1E`」，幂等；置位-清位-再置位在同 tick 内等价末态 |
| 队列永不清空（长忙帧） | 旗保持到队列清空的下一 tick；`apply_deferred_screen_on` 只在 `iter_gates.is_empty()` 分支执行（`crates/game-core/src/frame/engine.rs:177`） | 旗**不会丢失**（不清空就一直留着），只延后；实测暗窗 7 帧内队列正常排空 |
| 在 title 相位触发 | 旗只由 `more_init` 置位（`|0x1E` 写点在 crates/ 内**唯一一处** = `mainloop.rs:3048`）；title 相位走 `title_bridge.rs:492-493` 的 ScreenNmiOff（不同路径） | **无旗残留**；但见 F4：more_init 内 ScreenOff 的影子清位未建模 |
| `apply` 是否会被 span 分支绕过 | `end_of_tick_gate()` 在 `tick()` 两条臂尾**都无条件调用**（`engine.rs:148` AwaitNmi 臂、`engine.rs:163` PostSpin/MainLoop 臂）；span 臂只减 `iter_span_left` 后**仍调 gate** | **不会被绕过**。唯一跳过情形是 AwaitNmi 臂 `NMI_STATUS == 0` 时整臂不进 —— 此时 tick 未推进迭代、旗保持到下一 tick，不丢失 |

### D4 EndVRAMStruct 取模 vs asm 溢出行为

见 §F3：**不等价**（`&0x7F` vs `cpx #$4F` + 回卷段首），但本路径 `idx+4 ≤ 0x4F` 恒不触发，**当前无像素后果**；越界时 rust 会写到 `$07F1`（缓冲尾之外）。属潜在越界写，建议最小整改。

### D5 f700 / f1100 的 71/90px 定性

见 §F2：**不成立**（实施方「渲染相位族 / 错位 2 帧 / CHR 低址垃圾段」三种表述均被证伪）。真实定性 = **emu 截图与 emu dump 亚帧不同步**（emu 侧截图中多一层已消失的精灵），分类 **oracle/门禁口径问题**，不是移植错。

证据链闭合：
1. CHR/NT/PAL 段双侧逐字节 diff=0（14 帧全对比，`ppu.hex` 每侧 12323 B）。
2. OAM `$0200-$02FF` 在 f700/f1100 两侧 diff=0，且 64 精灵 Y 全 ≥0xEF。
3. 差异像素 71/90 全部落在 2 个 tile，颜色 `0x16=(181,49,32)` 在密码屏期只存在于精灵调色板。
4. `|rustPNG − render(rustDump)| = 0`，`|emuPNG − render(emuDump)| = 71/90`（与 L4 差逐数字相等）。
5. `/tmp/lag.py` 排除整帧滞后（f750 只降到 67；f1100 与帧号无关）。
6. 若「错 2 帧」成立，两侧 dump 必须不同 ⇒ 与 1 矛盾。

**边界声明**：未定位到 Mesen2 API 内部（哪个回调相位 flush surface）。故只能断言「这 71/90px 不是 rust 的 PPU/渲染状态差」，**不能**断言 rust 渲染 100% 正确。

---

## 竞争假设与证伪记录

复核期间主仓库出现**未提交**的 `crates/game-core/src/frame/title_bridge.rs` 改动（`title_no_return` 的 match 分支加入 `0x17`），其注释主张：「该相位跨 3 帧只 inc 1 次 FrameCount（emu 逐帧实录 f501=237 冻结 f502/503/504=237 ⇒ f505=238）…不做此冻结则 f505=240（+2 恒偏，使 f700/f1100 取景偏 2 帧=71/90px 假失配：emu f700 ≡ rust f698 整图 0px、emu f1100 ≡ rust f1098 0px）」。

**该假设事实部分成立、因果部分被证伪**：

- ✔ 成立：emu 确实在 f501-504 冻结 FrameCount（`analysis/traces/case-password-justin.frames.jsonl` 实录 f500=236, **f501=237, f502=237, f503=237, f504=237**, f505=238）；rust 侧 `$2D` 也存在相位差（f530..f1150 恒 +2、f1190 起恒 +4，实测）。
- ✘ 被证伪：「f700/f1100 取景偏 2 帧 ⇒ 71/90px」不成立。若是取景错位，**两侧 dump 内容必然不同**；实测 CHR/NT/PAL 逐字节 diff=0。且 §F2 已给出残差的唯一来源（精灵层在 emu 截图里多余）。
- ✘ 连带被证伪：`title/mod.rs:1258-1261` 的「垃圾段写到 CHR 低址 ⇒ f700/f1100 各 70/71B CHR 失配」——CHR 段两侧逐字节相等。

⇒ **不得以「帧号错位」为由修改 rust 的 FrameCount 调度**；那会引入真实的行为偏离且不消除残差。若要消除 71/90px，需修 harness 相位（§F2 整改 2）。

---

## 结论

- **Verdict：needs_revision**
- **三个 commit 的代码改动本身没有发现移植错**：asm/ROM 逐行对读与实施方语义一致（影子写 vs 真寄存器落点 ✔、点亮在下一帧 ✔、终止符需补 `$00` ✔），干净环境像素验收 `match 120 / diff 2 / blocked 0`，`cargo test` 536/0，无门禁削弱。
- **但三处叙事/契约问题必须整改**，按严重度：

| # | 严重度 | 整改项 |
|---|---|---|
| F1 | **高** | 补锚历史 10 个未锚采集点（jumpledge 7 + walkfcprobe 3，全 match 0px），使 `gates check` 的 l4 转 PASS；并更新验收期望口径为 122 格 / match 120 / diff 2 |
| F2 | **高** | 修正 `title/mod.rs:1258-1261` 与 `title_bridge.rs`（未提交）注释里的错误根因叙事（CHR 垃圾段 / 错位 2 帧）；71/90px 定性为「emu 截图与 dump 亚帧不同步」；正确修法在 harness 侧加同相自检 |
| F3 | 中 | EndVRAMStruct 终止符改为 asm 等价语义（`idx+4 >= 0x4F` 时写段首），或注释显式标注不等价 + 不可达证明；加 idx=0x4C 溢出路径单测 |
| F4 | 低-中 | `more_init` 补 ScreenOff 的影子清位（`$FE &= ~0x18`），或显式注释代偿路径 |
| F5 | 低 | 并发核查改在独立 worktree；`kill_orphan_mesen` 的 `pgrep -f "tools/mesen2/Mesen"` 加 worktree/display 过滤，避免同机多 worktree 互杀 |
| F6 | 低 | anchor history 的 `evidence` 字段补证据指针（历史惯例，非本轮引入） |
| F7 | 低 | `deferred_screen_on_inert_without_flag` 弱断言加固（显式设非默认 `$FE`/`ppumask` + `write_io` 计数不变） |

- **禁止事项（供后续 agent）**：不得为消除 l4 FAIL 而改阈值/扩白名单/删用例/删格；不得以「帧号错位」为由改 rust FrameCount 调度；不得把 f700/f1100 的 71/90px 当作移植错的证据继续改渲染路径。

---

## 复现命令清单

```bash
# 隔离 worktree（避免与并发 agent 互杀）
git worktree add /tmp/verify-r9 f7199a74
cp "rom/Metroid (USA).nes" /tmp/verify-r9/rom/
cd /tmp/verify-r9 && export CARGO_TARGET_DIR=/tmp/verify-r9-target

# C1
cargo test --workspace 2>&1 | grep -E "^test result" | awk '{p+=$4; f+=$6} END {print "TOTAL passed="p" failed="f}'
grep -rn "#\[test\]" crates/ --include=*.rs | wc -l

# C2 / C3
python3 tools/l4_diff_run.py run case-password-justin
python3 tools/l4_diff_run.py run-all
python3 -c "import json;m=json.load(open('shots_rust/matrix.json'));print({k:m[k] for k in ('cases_total','cells_total','match','diff','blocked','flaky','verdict')})"

# C4（约 30 分钟；内部会发起 run-all，注意与并发 L4 互杀）
python3 gates/cli.py check > /tmp/check-r9.log 2>&1; grep -E "^(SUMMARY|CHECK:)" /tmp/check-r9.log

# B 面
git diff 863f258a..f7199a74 -- gates/ | wc -l
git diff 863f258a..f7199a74 --stat
(cd /home/peng/workspace/metroid-decomp && sha256sum -c data/status/baselines/SHA256SUMS)
git grep -o "unported!" 863f258a -- crates | wc -l; git grep -o "unported!" f7199a74 -- crates | wc -l
git show 863f258a:cases/case-jumpledge-probe.txt | grep -c "^capture"   # 8
git show 863f258a:data/status/l4_ratchet_baseline.json                  # jumpledge 只 1 格

# D5 像素判定（脚本内容见正文，PNG 反解调色板 + 纯 BG 渲染器逐像素对比）
# /tmp/cmp2.py  : |emuPNG - render(emuDump)| 与 |rustPNG - render(rustDump)|
# /tmp/mask.py  : 差异像素按 8x8 tile 分组 + 掩码字形
# /tmp/lag.py   : 与所有可用 dump 帧两两求差，排除整帧滞后
# /tmp/spr.py   : 叠加精灵层（本帧/前帧 OAM、$0000/$1000 base）尝试复现 emu PNG
```
