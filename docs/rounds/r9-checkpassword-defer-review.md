# r9 复核档：CheckPassword 链延一拍（be11ab9b）——独立对抗复核

- **复核对象**：`be11ab9b` fix(core): CheckPassword 链延一拍——emu UnscramblePassword 忙跨相位建模（HEAD，parent=`abd7f72a`）
- **复核角色**：verifier（证伪立场，独立复跑，不盲信提交方验证段）
- **复核时间**：2026-11-12
- **Verdict：needs_revision**（代码面无放宽、无回归、链拍对齐为 A 级成立；但 commit message「验证」段对 f1190 的修复宣称被父本对照证伪——像素零变化、2063px 稳定复现，且真实像素根因未诊断）
- **Confidence：85**（像素零变化、链完成拍对齐、门禁全绿为直接实测；ScreenOff 归因与 70B 归因为 B 级推断）

---

## 问题清单（verdict 依据，均带 A 级证据指针）

### F1（主因）：commit「验证」宣称 f1190 已修复，实测像素零变化

- 提交方原文（commit message 验证段，verbatim）：「问题：case-password-justin f1190 submit **2063px**」→「验证：… L4 case-password-justin **f1190 CHR 形态修复**（SUITLESS→未载，与 emu JIB=0 相位对齐），**残留 70B**=emu title 期已载 SAMUS/rust 未载（f700/f1100 同族，下一卡）」。
- **父本对照（决定性 A 级）**：`git worktree add /tmp/m5d-abd7 abd7f72a` 后在父本跑 `python3 tools/l4_diff_run.py run case-password-justin --skip-emu`（同 scenario/rom/sched 三 sha 锚）：
  - 父本 f1190 = **2063px**，与 HEAD 相同；
  - **父本与 HEAD 全部 12 张 rust 采集 PNG 逐 md5 相同**（含 `case-password-justin_f01190_submit.png`）→ **本 commit 在本用例 0 像素变化**；
  - 我方两轮独立复跑（`run-all` 与单用例 `run case-password-justin`）f1190 均 = **2063**，非 flaky。
- 即：commit 以「问题：2063px」开头、以「修复…残留 70B」收尾，读者必然理解为该格像素已从 2063 降到 ~70；**实测 L4 验收指标（像素）2063→2063，一字未动**。矩阵行「match 107 diff 3 与基线同形」技术上诚实（f1190 仍在 diff 格），但验证段未披露「像素仍 2063」。
- 「残留 70B」在字节口径上**可复现但归因错**：HEAD f1190 的 emu↔rust ppu.hex 残差 = **71 字节**（12323B 文件，diff offset 集 = 0x0001-0007、0x000c + 自 0x001f 起每 0x20 一字节）；**同一 71B 族在 f700/f1100/f1150 完全相同，而 f1150 是像素 match 格** → 该 71B 是与提交无关的常驻残差族（palette/名表右缘），不是「emu 已载 SAMUS/rust 未载」的专属证据（f700 在提交动作之前就存在同族 71B）。
- **修复的真实收益（A 级，状态面）**：f1190 ppu 残差 emu↔rust **1086B（父本）→ 71B（HEAD）**，ram 残差 **88→59B**——链延拍确实把 rust 的 f1190 VRAM 拉回与 emu 同相。此点 commit 未夸大，但它是状态面收益，不是像素收益。

### F2：2063px 的真实根因（渲染开关相位）未诊断，commit 的 JIB/CHR 根因叙事被证伪

- **emu f1190 黑屏 = 渲染关，不是 CHR 差**（A 级）：`shots/case-password-justin_f01190_submit.png`（258B）唯一颜色 `[0,0,0]`；而 `analysis/traces/case-password-justin_f01190_submit.ppu.hex` **≡** `f01150_pw-done.ppu.hex`（内容相等，密码屏 VRAM 原封）→ emu 在 f1190 关渲染（ScreenOff/forced blank），VRAM 未变。rust f1190 渲染开 → 密码屏白字 2063 非黑像素（bbox y32-190 x32-230）。
- **JIB/CHR 相位对该格像素贡献为 0**（A 级）：父本 rust f1190 = post-chain（JIB=1、SUITLESS 已载、与 emu ppu 差 1086B），HEAD rust f1190 = pre-chain（与 emu ppu 差 71B）——**两种内部状态渲染出逐字节相同的 PNG**（均密码屏）。故 commit 根因句「rust 原同拍一气呵成 → JIB=1 装 SUITLESS（2063px）」对像素差的因果关系不成立：修掉它 0 像素变化。
- 真实可见差异 = **emu 渲染关 vs rust 渲染开**。ScreenOff 在 asm 上应发生在跳 `CheckPassword`（`prg0_title.asm:1718-1733`，本体无 PPU 写）之前的相位臂；rust 侧置 pending 的同拍 dispatch（`crates/game-core/src/frame/title_bridge.rs:166`）未执行渲染关事件（B 级推断，需下一卡取证确切 asm 位置）。
- 附带状态面证据（A 级 ram dump）：f1190 `JOY1_CHANGE($0012)` emu=0x10（仍持 START、转场进行中）rust=0x00（尚未进转场）——emu 在 f1190 已处于 submit 转场中态，rust 仍 pre-chain。

### F3（要求修订动作，非回滚代码）

1. 更正 commit message/复核叙事：明确「f1190 像素仍 2063，本 commit 像素零变化（父本对照 12/12 PNG md5 相同），收益在状态面（ppu 残差 1086→71B）」；把「残留 70B」标注为 ppu.hex 字节口径且修正 SAMUS 归因（同族 71B 在 match 格 f1150 也存在）。
2. 下一卡目标改为真实像素根因：emu f1190 渲染关（ScreenOff 相位）rust 未建模——按 §4 三问先取证 asm 位置再动手。
3. （可选加固）在 L4 增加 f1191 采集点钉住链完成拍（当前 1190→1300 无中间点，±1 拍误差不可见，见遗留假设 L1/L3）。

---

## A 面：asm/ROM 逐行对读——「忙跨 2 tick / 落延一拍」忠实性

**结论：对 JUSTIN BAILEY 忠实（链完成拍 A 级对齐、恰延 1 拍）；模型对其他密码的泛化未取证（遗留 L1）。**

- asm 对读（`refs/m1disasm/SRC/prg0_title.asm`）：`CheckPassword:1718-1733`（00:8C5E）= jsr ConsolidatePassword → jsr ValidatePassword → bcs L8C69 / jmp InitializeGame($92D4)；`ValidatePassword:1994`（00:8DDE）= NARPASSWORD 已置→跳过；NARPASSWORDTbl(16B) 命中→置旗 valid（**不进 Unscramble**）；否则 jsr `UnscramblePassword($8E4E)`→PasswordChecksum($8E21)→比对；`UnscramblePassword:2078`（00:8E4E）= `lda PasswordByte+$10; sta Temp02` 外层 @loop_A(8E53) × 内层 @loop_B(8E5A, `rol PasswordByte,x/dex/bpl` 16 次) + rol Temp00 + `dec Temp02/bne`——纯 CPU 忙循环，忙长 ≈ 221×count+~7 cycles（count=$6998 面），帧=29780 cycles。
- emu 忙时长（A 级，probe-pw-jib）：cputrace `8E4E` 入口 frameCount 1201 → `8E21/8E05/92D4` 于 1203（frameCount−harness 偏移 12 → harness f1189 入口、f1191 完成）；frames.jsonl：START 边沿首现 f1189、JIB($69B3)=0 于 f1190、**JIB=1 首现 f1191**；FC 冻结 f1189-1191。→ **emu 链完成 = START+2 拍，忙跨 2 tick 对本密码成立**。
- rust 链拍（A 级，本次新探针）：向 `shots_rust/case-password-justin.sched` 插 `capture 1191 probe1191`/`capture 1192 probe1192`，`target/debug/rust_screenshot --schedule /tmp/pw1191.sched --out /tmp/pw1191cap`：
  - f1190 采集仍 pre-chain（与 HEAD f1190 相同）；
  - **f1191 ppu.hex ≡ 父本 f1190 post-chain ppu.hex（0 差）→ rust 链恰在 f1191 执行**；
  - f1192 PNG 已进入转场（11431 非黑、9 色）。
- 对齐判定：**rust 链完成拍 f1191 = emu 链完成拍 f1191（JIB=1 首现）→ 延 1 拍恰好、不过量不足**（对本密码）。父本（pre-fix）链在 f1190 = 早 1 拍，fix 恰 +1。
- 模型简化（B 级，记录不否决）：延拍是**固定 1 拍**，而 emu 忙长随 count 连续变化（≤~134 同帧完成、count=0 反而 256 次最忙）且 NAR 路径零忙——即该 1 拍实为「emu busy=2 拍」在本密码上的拟合，非从忙长推导。

## B 面：防弱化审计——无放宽

- `git show --stat be11ab9b`：**仅 4 文件**——`crates/game-core/src/frame/title_bridge.rs` +76/-25、`crates/game-core/src/state.rs` +7、`crates/game-core/tests/password_init_parity.rs` +3/-1、`tools/scenario_run.py` +45。**`gates/` 与 `data/status/` 零改动**；git status 复核时干净（HEAD=be11ab9be700cf6b4e19e54663383706124f9ced）。
- 测试改动逐个判定 = **适配新语义 + 加锚，非削弱**：
  - `password_init_parity.rs` run_one：原断言不动，仅在断言前补第二次 `main_title_routine(&mut m)`（首拍置 pending/第二拍执行链）；102 组对拍语义不变。
  - `title_bridge.rs` 两测：原断言（want_pb 18 字节、`SFX&1==1`、`TIMER3==0x0C`、`TITLE_ROUTINE==0x18`）**全保留**，前置新增 `assert!(m.check_password_pending)`（首拍）与 `assert!(!m.check_password_pending)`（次拍）——若有人回退延拍机制（首拍即执行链），`assert!(pending)` 立即爆 → 是防回退锚，不是弱化。
- `tools/scenario_run.py`：新增 `cputrace/ctread/ctwrite` DSL 解析（4 位 hex 校验、`int(ad,16)` 存 int——修 lua:342 `string.format %04X` 类型错、去重）+ `ctwindow`（十进制闭区间、w0≤w1 校验）+ defaults 4 键。**纯加法，不碰采集/锚/判定逻辑 → 无放宽**。
- 未动阈值/白名单/基线：`data/status/l4_ratchet_baseline.json` 本 commit 未改。

## C 面：独立复跑（实测输出原文）

1. `cargo test --workspace` → **532 passed / 0 failed**（各二进制逐行求和，exit 0）。
   - 门禁自身口径：`[PASS] rust-tests …测试数 536（基线 536）`（与直跑差 4，计数口径不同，两者均 0 failed；测试数与基线持平=未删测）。
2. `python3 gates/cli.py check` →
   ```
   SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2
   CHECK: PASS（含 0 个静默跳过）
   [PASS] l4 …用例 14 · 分母 110（blocked 0 计分母不计 pass）｜match 107 diff 3 blocked 0｜白名单条目 0
   ```
   （TOLERATED=2：trace-diff `l3-samus-move3-oracle-determinism` oracle 缓存缺损 1 项；visual-diff 45 失败项账内容忍——均为存量，非本 commit 引入。）
3. `python3 tools/l4_diff_run.py run-all` → **用例 14 · 格 110 · match 107 · diff 3 · blocked 0 · flaky 10**（flaky=case-jumpledge-probe 7 格+case-walkfcprobe 3 格，多 run 记录不一致，不计分母）。
   - **逐格对照基线：match→非match 回归 = 0**。
   - 现存 3 个 diff 格（case-password-justin）：700:mid-1=**71**（基线 1365）、1100:mid-4=**90**（基线 1384）、1190:submit=**2063**（基线 769）。700/1100 的改善来自更早 commit（基线文件较旧）；**1190 的 769→2063 发生在本 commit 之前**（简报 pre-commit 状态即 2063），像素数不受 ratchet 守卫（基线只锚 match 集）。
4. `python3 tools/l4_diff_run.py run case-password-justin`（单用例复跑）→ f1190=**2063**（与 run-all 一致，稳定非 flaky）。
5. 父本对照（F1 证据）：`/tmp/m5d-abd7`@abd7f72a `run case-password-justin --skip-emu` → f1190=**2063**，12/12 rust PNG md5 与 HEAD 相同；ppu 残差 emu-parent=1086 / emu-HEAD=71 / HEAD-parent=1116；ram 残差 88/59/49。
6. f1191/f1192 探针（A 面证据）：见 A 面——f1191 ppu ≡ 父本 f1190 post-chain（0 差）。

**复跑基线纯净性说明**：run-all/cargo test（≤11:36）与 11:39 的干净 git status 均在外部并发编辑出现之前 = 纯 be11ab9b 树。复核期间仓库出现**非本 commit、非复核员所为**的并发编辑：`M crates/game-core/src/frame/memory.rs`(+16, mtime 11:40:27)、`M crates/game-core/src/frame/nmi.rs`(+3, mtime 11:45:48)，内容为 `DSH_CHRW_TRACE` env 门控的 CHR 写打点（注释「f700 70B 污染定位，用完删」，无 env 即 no-op）；`target/debug/rust_screenshot` 于 11:45:49 重编。我 11:56 的 f1191 探针使用该二进制（no-op 打点，语义等价）；11:41 单用例复跑在两编辑之间，结果仍 2063。**建议 lead 知悉并确保该临时打点不入库。**

## D 面：pending 机制新问题扫描

- **消费点唯一性**（grep 全量）：消费者仅 `title_bridge.rs:37-38`（spin 检查后、dispatch 前，消费即 `return` 早退、不置 `title_no_return`=变体 B）；置位仅 `:166`；`main_title_routine` 调用点 `frame/mainloop.rs:3164`、`frame/engine.rs:240`。无第二消费点/无双写。
- **spin 挡消费（理论边，本用例未爆）**：消费判定在 `title_spin>0` 早退**之后**——若置 pending 的下一拍恰逢 `title_spin>0`，spin 先 return，pending 顺延 ≥1 拍。本用例 f1191 spin=0（链准点执行，A 级实测）→ 未触发；属未证伪的边界（L4 无该组合用例）。
- **title_no_return/FC 自增**：消费拍 `title_no_return = matches!(TITLE_ROUTINE, 0x00|0x01|0x15|0x1B)`——本用例 TITLE_ROUTINE=0x18 → false → FC 照常自增。emu 侧 FC 冻结 f1189-1191，**FC 相位差（$002D：f700 emu=0xB1/rust=0xB3 +2；f1190 emu=0x99/rust=0x9D +4；父本 f1190 同 0x9D）为 pre-existing，本 commit 既未引入也未修复**；本用例无 FC 看门（fcprobe 用例独立且 flaky），差值至 f1300 后 emu 侧 FC 复位（emu=0x00/rust=0x04）仍 +4。变体 A（置位）已由提交方 A/B 证伪（f1430 169px NT B 未写回归，我方复跑 f1430=match 佐证变体 B 选择）。
- **二次 START 吞噬**：pending 只消费一次；消费拍早退跳过 `title_tick_head` 的 START 检查 → 消费拍上的新 START 顺延到下一拍处理。emu 忙循环期间主循环同样不读输入 → 方向一致，未见吞噬缺陷。本用例 joy 表仅 1188/1189 两拍 START，无三连按用例。
- **invalid 路径延拍**：链体含错码三写（`SFX_MULTI_INIT|=1`、`TIMER3=0x0C`、`TITLE_ROUTINE=0x18`）整体延一拍；emu invalid 路同样经 Unscramble 忙 → 延拍方向正确（title_bridge.rs:424-426 有第二拍 invalid 测试锚，断言保留）。但同受 L1 的 count/NAR 依赖。
- **emu mid-chain 态不可表达**：f1190 emu 处于链进行中（JOY1_CHANGE=0x10、渲染已关、59B ram 差），rust 为原子 pre-chain——tick 粒度原子链模型固有残差；本窗口像素不可见（f1300 match），登记为状态面残留。
- **无 pending 饿死路径**：`main_title_routine` 每 tick 由 mainloop 调用，pending 置位后下一 tick 必达消费点（本用例 f1190 置 → f1191 消费，A 级）。

---

## 遗留假设清单

- **L1**：延拍对「零忙路径」（NARPASSWORD 已置 / NARPASSWORDTbl 命中，`prg0_title.asm:1994` 跳过 Unscramble）与小 count 密码（count≤~134 同帧完成）可能过量 1-2 拍——L4 仅测 JUSTIN BAILEY 一密码；`password_init_parity` 102 组只验双拍语义自洽，无 emu 交叉。需扩 emu 密码矩阵取证。
- **L2**：「70B」为 ppu.hex 字节口径（实测 71B），且该族残差在 match 格 f1150 同样存在——「emu 已载 SAMUS/rust 未载」归因为 B 级存疑。
- **L3**：f1190→f1300 之间无 emu 采集点——emu 渲染何时开、rust 渲染何时关、链完成拍 ±1 拍的像素后果均未逐帧钉住（f1191/f1192 仅 rust 侧探针）。
- **L4**：emu ScreenOff 的确切 asm 位置（相位臂 vs CheckPassword 前置）未取证；F2 的「rust 未建模 ScreenOff」为 B 级推断。
- **L5**：FC 相位差 +4 pre-existing 未建模（emu FC 冻结实录 vs rust 每拍自增），超出本 commit 范围，后续需 FC 看门用例。
- **L6**：复核期间存在并发外部编辑（memory.rs/nmi.rs 的 DSH_CHRW_TRACE 临时打点）——本档全部复跑数据取自编辑前树或 no-op 二进制；该打点不应入库。
- **L7**：`spin>0` 与 pending 同拍的组合未被任何用例覆盖（理论顺延边）。

## 复现命令

```bash
cargo test --workspace                          # 532 passed / 0 failed
python3 gates/cli.py check                      # SUMMARY: PASS=13 FAIL=0 SKIP=1 TOLERATED=2
python3 tools/l4_diff_run.py run-all            # 14 用例 · 110 格 · match 107 diff 3 blocked 0 · flaky 10
python3 tools/l4_diff_run.py run case-password-justin   # f1190=2063（稳定）
# 父本对照：
git worktree add /tmp/m5d-abd7 abd7f72a
#   链 rom/Metroid (USA).nes、拷 shots/case-password-justin* 与 analysis/traces/case-password-justin* 后：
cd /tmp/m5d-abd7 && python3 tools/l4_diff_run.py run case-password-justin --skip-emu   # f1190=2063，12/12 PNG md5=HEAD
# 链拍探针：
sed 's/^capture 1190 submit$/capture 1190 submit\ncapture 1191 probe1191\ncapture 1192 probe1192/' \
    shots_rust/case-password-justin.sched > /tmp/pw1191.sched
target/debug/rust_screenshot --schedule /tmp/pw1191.sched --out /tmp/pw1191cap   # f1191 ppu ≡ 父本 f1190
```

## 三审补充（lead 整改回执，2026-11-12）

> 【原地追加，不删上文】本档 verdict=needs_revision 的三条（F1/F2/F3）lead 全部认领，并给出整改与新证据。

**F3-① 验证叙事更正（认领）**：`be11ab9b` 的「2063px 修复」表述不成立——该 commit **像素零变化**（父本 vs HEAD 12 张 rust PNG 逐 md5 相同），真实收益=状态面（f1190 ppu 残差 1086B→71B、ram 88→59B）；「残留 70B」的归因撤回为「常驻残差族（同 offset 集亦见于 match 格 f1150）」。

**F3-② 真根因取证（新 A 级证据，修正 F2 的相位判读）**：
- emu 逐帧实测（`probe-pw-offwin`，capture 1180-1205 逐帧 + PNG 颜色数解码）：**渲染关窗口 = f1191–f1197（7 帧，258B/1 色全黑）**，**f1190 仍是密码屏（1516B/4 色）**。r9 依据的 `shots/case-password-justin_f01190_submit.png`（258B 全黑）是**采集相位晚一帧**的产物，不是 f1190 的真值。
- **真根因（比 F2 更基本）**：rust 渲染器**完全没有 PPUMASK 门控**——`$2001` 写不进 RenderState（`write_io` 无 0x2001 臂），`render_frame_bg` 无条件画 BG。emu 渲染关期间 rust 照画密码屏。
- 已修（本卡随附 commit）：`RenderState.ppumask` 字段 + `write_io(0x2001)` 落值（asm `WritePPUCtrl: ; 07:C44D` 的 `sta PPUMASK`，prg7_engine.asm:517）+ `render_frame_bg`（bit3=BG）/`render_frame`（bit4=精灵）门控 + 2 条 TDD（`render::tests_ppumask_gate::*`）。
- **未闭合（登记）**：门控落地后 f1190 仍 4 色（capture 时 `$FE=1E`——链在 tick1189 清、tick1190 又被 `more_init` 的 ScreenOn 恢复），即**转场持续时长差**：emu 从 ScreenNmiOff 到 ScreenOn 跨 7 帧（bank 切换/InitBank/MoreInit 调度），rust 一 tick 完成。此属转场调度族（与 BUG-089 掉帧窗同源），非渲染门控本身。

**F2 观感修正（保留其结论方向）**：r9 的「emu 渲染关」判断成立；但关窗起点是 f1191 而非 f1190，且 f1190 的 2063px 是「rust 画密码屏 vs emu 该采集点（晚一帧）黑屏」的**采集相位差**，不是 f1190 真值差。
