# r9 · FC 对齐判据对抗复核（只读）

> 复核对象：P0/P1.0/P1.0b/P1.0c（HEAD `70d3edfa`）。立场=证伪。
> 纪律：未改动任何实现文件；本文件是唯一写入。未跑全量房扫/L4/cargo test。

## 0. 一句话结论

**部分对。**「不建模掉帧、由 `$002D` 实测 Δ 驱动对齐」这个方向成立；但
**不能把 r31 的 ~700px 残差归因于"emu 显示晚一拍"并用缝条件吸收**——同一
`(FC,ScrollX)` 唯一缝上，背景整屏 0px、残差只落在两个精灵盒内（门 x28–46、
Kraid x180–226），这是**采集/OAM 相位问题（或 Rust 精灵相位真差）**，不是全局
显示滞后。把它记成 residual 会吃掉真差异。对齐只能先做**诊断面**，不得直接改
ratchet 判定。

## 1. Q1 方向与风险

### 1.1 三个方案对比

| 方案 | 代价 | 风险 | 裁定 |
|---|---|---|---|
| A 引擎建模掉帧 | 需 cycle 级主循环预算 + 丢帧时 PPU/NMI 抑制；等于把行为钉死在 Mesen 周期计数 | 与 D3「ROM 字节+Mesen 执行」冲突面扩大；Stage A 保真对象从「游戏逻辑」变成「某模拟器时序」 | 否 |
| B 不建模、按 `$002D` 对齐 | 中（见 §2） | 若对齐规则含像素项，会吸收真缺陷（现状已发生，见 1.2） | **是，但限诊断** |
| C 标 known-residual 不判 | 低 | 34 mismatch 房永久脱离判据；ratchet 分母缩水；等于放弃这些房 | 仅作最后退路 |

方向正确的核心理由：判据应对「游戏时间」负责，`$002D` 是 ROM 自己维护的游戏时间
（`prg7_engine.asm:207`，bank7 `C0C5 E6 2D`），比视频帧号更接近"行为等价"的语义。

### 1.2 关键风险：会不会吃真缺陷？——**已经吃了一次**

- `tools/movie_ref/room_sweep.py:790-800`：`best_pairing = min(same, aligned)`，
  `aligned` 是 rust 采集 c 对 emu `prev{c}`（`room_sweep.py:180-184`）的 diff。
  **这是"按像素取小"的对齐**，不是按状态对齐。P1 文档自述 156 个 ok 房里 22 个
  靠 `shift1` 过关——这就是吸收面。
- `room_sweep.py:802-806`：ok 时无条件写 `aligned=True, aligned_offset=-1`，
  与是否真的对齐无关（P1 文档也已判为 dead semantics）。台账在**说谎**。

**可判定的区分条件（必须机械满足，不能满足就 mismatch）：**

1. **位移量只能来自状态面**：`D(f) = FC_rust(f) − FC_emu(f)` 由 `$002D`
   （emu watch / rust `zp[0x2D]`）算出，**计算过程不得读像素**。禁止任何
   `min(diff)`、`best_pairing`、±1 搜索。
2. **配对唯一**：目标 `(FC,ScrollX)` 在 emu 缝表内恰有 1 行；>1 行 ⇒
   `fc_pair_ambiguous`，判 mismatch（不得取最小 diff 的那行）。
3. **配对点是真缝**：`SHOT-SEAM at == f`（非 `f+1` fallback）；fallback 帧的
   显示面是上一缝，不构成独立采样。
4. **配对后必须 0px**：`fc_diff_pixels == 0` 才可称 `ok_fc_aligned`；
   非 0 ⇒ 保持 mismatch。**位移是解释"帧号"，不是解释"像素"。**
5. **ratchet 键不动**：`data/status/l4_ratchet_baseline.json` 与房扫 status
   继续以严格同帧 0px 为准；FC 字段只做诊断列（即 P1 计划 D1）。

这 5 条能挡住"真缺陷被平移吸收"：真缺陷在状态推导出的唯一缝上仍非 0；而现状的
`min(same,aligned)` 挡不住，因为它允许用像素挑选解释。

### 1.3 缝条件可行性——**机制可行，但不能解决 700px**

- Lua 侧已有 `sample_point spin-exit`：`tools/scenario_run.py:386-391`、
  `tools/scenario_capture.lua:1128`（缺省 endFrame）、`:1220-1237`（$C0D3
  exec 回调、`pendingShot.frame==frame` 晚缝守卫）、`:1294`/`:1304-1310`
  （`frames+1` / 下一帧边界 fallback，日志 `SHOT-SEAM ... at=`）。
- Rust 侧已有同构缝钩子：`crates/game-core/src/frame/engine.rs:88-118`
  （NMI 后/工作前调 `on_spin_exit`），`rust_screenshot.rs:585-612` 在缝上写
  state 行与 `seam_snap`。
- **但当前采集不是单相位**：`rust_screenshot.rs:544-566` 采集帧 N 的
  png/ppu 用 `seam_snap`（key=N−1，pre-work），ram 用 `engine.mem` 活体
  （post-work）；`:415-423` 注释自认"双相位"。两腿"同一缝"没有保证。

**这是 700px 的第一嫌疑**：png 的背景面（nametable/scroll）来自 snap，
精灵面（OAM）也来自 snap；若 snap 的 OAM 比 RAM 相位旧一拍，则**背景完全一致、
只有精灵差一拍**——正是实测形态（见 §1.4）。代价：不需要新钩子，只需让同一次
采集的 png/ppu/ram/OAM 取同一相位（约 1 天，含 lag 帧回归），或在 capF 记
`shot_at` 后由比较器显式选相位。

### 1.4 独立复算：700px 不是显示滞后（证伪 P1.0c）

对 `shots_rust/case-p0c-r31_f01721_f121.png` vs `shots/case-p0c-r31_f01726_f126.png`：

- 全图 diff 699px；bbox = (32,80)–(221,183)，但**分簇**为 x32–41（门，230px）
  与 x186–221（Kraid，469px）。
- 背景带实测：x0–28 = 0，x46–180 = 0，x226–256 = 0（c=1721/1725 同）。
- 门盒 `R[75:190,28:46]` 与 emu 同盒**左移 2px**比较 diff = **0**（dx=−1/0 分别
  128/230）；即 rust 的门相对背景右移 2px。
- 同一 `(FC,ScrollX)` 规则下 c=1720→emu1724、c=1722→emu1727 均 **0px**，
  只有 c=1721/1725/1729 非 0。

推论：若是"emu 显示晚一拍"，滚动 2px/帧会让**整屏**平移 ≈11–14K px（相邻 emu
帧实测 10662–11652px），不可能只剩 699px 且背景 0px。**P1.0c 的归因不成立。**
残差要么是 rust 采集的 OAM 相位（§1.3），要么是 Rust 精灵动画真差一拍；两者都
不是"缝"能吸收的，必须定位后修或登记 bug，不能记 residual。

### 1.5 对 lead 事实清单的更正（证伪项）

- 事实 5「a0_r02 恒 −1」：按同文定义 `D=FC_rust−FC_emu`，实测 `/tmp/p0` 状态行
  f900 D=0、f903 起 D=+1；−1 是 `FC_emu−FC_rust`。且 D 在载入窗内**不是常量**
  （0→+1）。定义与符号必须钉死，否则"分段常量"会被误读。
- 事实 9「192/192 error 同源」：HEAD 已被 `13b3822f` 修掉（`room_sweep.py:173-186`
  有 `taken` 去重 + `f{c}>prev{c}>prevend` 优先级）。该条对 HEAD 已过期。
- 事实 8「P1.0c 定谳显示晚一拍」：见 §1.4，不成立（至少未定谳）。

## 2. Q2 实现代价

a) **二遍窗口**：`build_case` 现 `endFrame entry+frames`（`room_sweep.py:168-188`），
   采集点受 `:1048-1050`（1..frames）约束。要 `g ≥ c` 必须同时抬 `frames` 到
   `max(c)+Δ+margin`。**+10 不够**：P1 文档自测 a2_r29 `g≈c+13`。窗口应取
   `max_c(Δ_measured)+2`，Δ 由第一遍 `$002D` 行算，不能写死。代价：低（单房
   重跑路径已有 area/room 参数），风险：endFrame 行数变化会影响 `analyse_trace`
   的 `mr_window`/`entry..entry+6`（`:300-318`，窗口在开头，不受尾部延长影响）。

b) **状态面**：emu `watch 002D` 已有（`scenario_capture.lua:1195-1200`）；
   rust `--state-out` 行 = `{"f":N,"w":{"zp":"<hex>",...}}`
   （`rust_screenshot.rs:320-324` → `memory.rs:206-230`），FC 就是
   `zp[0x2D*2:0x2D*2+2]`，**两字符解码，3 行代码**，不构成阻塞。ScrollX 不在
   zp 面，需从同一行的 `pw`（PPU watch）或 RAM 镜像取；`l4_diff_run.py` 目前
   **不传 `--state-out`**（`execute_case :1024`），要加一个参数透传。

c) **唯一配对**：endFrame 下同 FC 相邻帧已实测（1724/1725 同 164、1729/1730
   同 168），所以 endFrame 不能做唯一键；spin-exit 一缝一行（P1 文档实测）。
   但改 `sample_point` 会破 `analyse_trace` 的逐帧 `mr_window`
   （`room_sweep.py:306-313`）与 `ENTRY_FRAME-1`（`:295-297`），需**双 trace**
   （身份判定仍用 endFrame，配对用 spin-exit）或在同一 run 里双回调采样。
   多候选策略：fail-closed（`fc_pair_ambiguous` ⇒ mismatch），**禁止取最小 diff**。

d) **台账 schema**：`gates/` 全仓无 `room_sweep` 引用（grep 0 命中）；L4 ratchet
   只吃 `l4_diff_run` 矩阵状态（`gates/cli.py:1051-1057`、`:1369`），不读房扫
   字段 ⇒ 加 `ok_fc_aligned/fc_deficit/fc_shift` **不会撞门禁**。但要注意：
   `room_sweep.py:896-925` 的 `planned==ran+skipped`、`ran==Σjudged` 自洽算术
   必须把新状态计入，否则房扫自身会报不一致；且把 34 房从 mismatch 翻 ok 会改
   L4 基线语义（ratchet 只升不降允许"变好"，但等于把吸收固化，须单独 commit
   写明影响，AGENTS §2）。

e) **回归红线**：Brinstar 47 / Norfair 44 的 Δ 实测 ≡0，机械验证 =
   (i) `fc_align` 纯函数单测：`deficit==0 ⇒ 恒等映射`；
   (ii) 只重跑这两房，断言 `status/diff_pixels/fc_*` 字段与改前逐字节相同；
   (iii) 断言 `data/status/l4_ratchet_baseline.json` 未被这批改动触碰。

**最坏情况**：700px 无法用缝消掉（按 §1.4 基本确定）。此时 FC 路仍成立，
但只到"诊断"层：`status` 保持 mismatch，新增 `fc_deficit/fc_pair/fc_diff`
解释差异；残差单开 bug 卡（嫌疑：`rust_screenshot.rs:544-566` OAM/状态双相位，
或 Rust 精灵动画相位）。退路 = P1 计划 D1；**不得用 whitelist/residual 把
699px 抹平**（AGENTS §5 白名单唯一例外面）。

## 3. Q3 判断

- **一句话**：**部分对**——"不建模掉帧 + 状态驱动对齐"对；"缝条件吸收 700px"
  不对；对齐只能先做诊断，不得直接改 ratchet。
- **本周可行性**：**诊断面可落地（约 2–3 人日）**：①`--state-out` 透传 + `$2D`
  解码；②`fc_align.py` 纯函数（唯一配对、fail-closed）；③台账加 3 个诊断字段
  并保持 `status` 不变；④两房红线。**转 ok 不应本周落**。
- **最大阻塞点**：699px 的归属未定谳。决定性实验很便宜（单房）：
  对 r31 把采集改成单相位（采集帧 N 的 png/ppu/OAM 全用活体 post-work，或
  全用 snap），重跑 c=1721/1725/1729，看 `699/718/697` 是否归零。
  归零 ⇒ 采集仪器 bug（修改 `rust_screenshot.rs:544-566`，不碰游戏逻辑）；
  不归零 ⇒ Rust 精灵相位真差，开 bug 卡，FC 路只做诊断。

## 4. 证据锚点汇总

- `prg7_engine.asm:207` / bank7 `C0C5 E6 2D`：FC 在主循环尾自增。
- `engine.rs:88-118`：NMI→缝回调→work；`:207` watch 面。
- `rust_screenshot.rs:415-423`、`:544-566`、`:585-612`、`:320-324`：采集双相位。
- `scenario_capture.lua:1128`、`:1220-1237`、`:1294`、`:1304-1310`：缝采样/fallback。
- `scenario_run.py:386-391`：`sample_point spin-exit|endFrame`。
- `room_sweep.py:168-188`（build_case/去重）、`:300-318`（mr_window）、
  `:760-807`（判定与 `min(same,aligned)`）、`:896-925`（算术）、`:1048-1050`。
- `l4_diff_run.py:352`（零容差）、`:1024`（execute_case，未传 state_out）。
- `gates/cli.py:1051-1057`、`:1369`（ratchet 只读 l4 矩阵）。
- 实测：`shots_rust/case-p0c-r31_f01721_f121.png` vs
  `shots/case-p0c-r31_f01726_f126.png`（699px；背景 0；门盒 dx=−2 为 0）。
