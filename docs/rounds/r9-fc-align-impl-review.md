# r9 · `tools/movie_ref/fc_align.py` 实现对抗复核

- 复核对象：`9681af9d`（`tools/movie_ref/fc_align.py`，230 行）
- 立场：证伪（不采信提交说明，重导证据）
- 纪律：只读；唯一写入 = 本文件。未跑 `room_sweep.py` 全量 / `gates/cli.py l4` / `cargo test`。
  允许的 `python3 -c` 小检查已执行，命令见 §4。
- 定位前提：本工具是**诊断面**，不参与 ratchet 判定（提交说明与 `docs/superpowers/plans/2026-11-23-fc-align-p1-implementation.md` P1.0f 一致）。

---

## 0. 结论（先看）

| 问 | 定谳 |
|---|---|
| Q1 判据合法性 | **合法（限诊断），但只能作假设、不能升格为判据**：选择器是 emu 单侧的显示面谓词，与 rust 像素无关，不构成 `min(diff)`；实测它在 1729 选了**更差**的产物（24818 vs 697）正好证明它不是挑小。但它是在 P1.0f 同一张表上归纳出来的（含 1 个手工豁免），无出样本验证。真正的「吃缺陷」通道不在相位，而在采集点识别（Q2-e）。 |
| Q2 实现 | 7 条里 **5 条有实锤缺陷**、2 条为限制。最重：采集点识别把 `prev*`/`pre`/`prevend` 当判定点（实测一房 4/7 行来自参考帧，且 4 个 `ok` 全部是参考帧）；失败行不进 summary 且退出码恒 0；载房窗无机械源时 1729 会被判 live。 |
| Q3 verdict | **需改后可用**（作为诊断面）。三条必须改见 §3。 |

---

## Q1 判据合法性：不是 `min(diff)`，但也不是零成本

### 1.1 形式化区别（可判定）

设 rust 每采点有两份预存产物 `seam_c`、`live_c`，emu 配对帧 `g`。

- `min(diff)`：`d = min(px(seam_c, emu_g), px(live_c, emu_g))`。选择器**读 rust 像素**，每点自由度 2；由构造 `d ≤ 任一单相位`，即单调偏向更小值 ⇒ 任何「两相位里有一个碰巧接近」的内容差都会被吃成 0。
- 本实现（`fc_align.py:118-122`）：`phase = f(emu(g-1) == emu(g))`，`f` 只读 **emu 自己的相邻显示帧**（`fc_align.py:85-98`），完全不读 rust 像素；选定后再算一次差。选择器在 rust 像素上**常量**，每点自由度 0。

**可判定判据**：若某实现的选择结果与 rust 像素无关，则它不可能是 `min`。本实现满足；且下列实测给出正向反证：

- 复核实测 · 用 `case-p0c-r31` 的 seam 产物 + `--live-dir` 缺失跑 `align_case`：
  `c=1729 → g=1736 → phase=live`（`/tmp/empty_live` 缺图 → `missing_png`）。
  而 P1.0f 表同点：`seam diff=697`、`live diff=24818`。
  ⇒ 规则主动选了**差 35 倍**的那份；`min(diff)` 绝不会这样选。

### 1.2 但归纳来源是同一批像素（必须声明）

P1.0f 的规则表（plan `:501-540`）就是「seam diff / live diff 两列 + emu 重复帧」归纳出来的：10 帧里 9 帧命中，唯一例外 c=1729 用「载房窗」领域知识手工豁免。因此：

- 在 r31 上它不是预测，是**拟合**；`--room-load` 又是人工帧表（`fc_align.py:200,209`），例外分支同样是人给的。
- 出样本（另一 mismatch 房，规则冻结后先跑）**未做**。在此之前它只能产出「诊断提示」，不能进 verdict；当前提交也确实是这么定位的（提交说明「诊断面，不动 ratchet」）——这一点**成立**。

### 1.3 结论

**合法**，因为错误方向是单侧的：相位选错 ⇒ 差变大（假失败），不会把内容差吃成 0；唯一能把 0 写进 `ok` 的通道是配对/采集点错误（见 Q2-e），与「按像素挑对齐」是两件事。**但它没有出样本证据，且例外分支靠人工；升格为判据前必须补一条 out-of-sample**。任务里说的「和 `min(diff)` 的本质差别」= 选择器上界（oracle 单侧 vs 双腿像素）；「没有差别的地方」= 两者最终都落在同一批 P1.0f 数据上被检验过，rule 的 9/10 是拟合值，不是泛化值。


## Q2 逐条缺陷

### (a) `horizon=40` 与「静默 no_candidate」——**缺陷（失败可见性），窗口本身目前够**

- `pair_frame` 只搜 `[rust.frame, rust.frame+horizon]`（`fc_align.py:106-110`），单向；docstring `:102` 明说 `g >= rust.frame`。
- 实测 lag 上界（`case-p0c-r31`，24 个采点）：`lag∈{0,4,5,6,7,8}`，max=8；`case-p0b-r31` max=6。**40 有 5× 余量，当前够**。plan §1.2 的 `D=−1`（a0 f784）出现在载房窗、非采点，未观察到采点需要 `g<c`；若出现，单向窗会直接 `no_candidate`（失败方向安全，但静默）。
- **真正的缺陷是静默**：`no_candidate` 只写成 `{"error": "no_candidate"}` 行（`fc_align.py:171-173`），`ok/nonzero/max` 从 `diff is not None` 里取（`:181-188`），`main` 只打印 `diff` 非零行（`:224`），**恒返回 0**（`:226`）。
  实测 `case-p0-0-02-end`：`frames=7 ok=4 nonzero=0 max=0`，其中 1 行 `no_candidate`、2 行 `missing_png` —— 汇总看起来「全绿」。
- 另：配对不以上限「可用 emu PNG」约束，会把 `g` 配到采集窗之外。实测 `case-p0c-r31`：c=1733..1740 → g=1741..1748，emu 目录只到 1736 ⇒ 8 行 `missing_png`，这些点被无声丢出统计。

定谳：窗口数值可放过；**静默必须改**（进 summary + 非零退出码 + 区分「数据没采到」与「真差」）。

### (b) `parse_rust_state` 硬编码 `zp` 偏移 —— **缺陷（输入未校验；有静默错值路径）**

- `FC_ADDR=0x2D`、`SCROLL_X_ADDR=0xFD`（`fc_align.py:32-33`），`zp` 切片 `:79-80`。
- 权威锚在 `crates/game-core/src/ram_map.rs:50`（`FRAME_COUNT: u16 = 0x002D`）与 `:66`（`SCROLL_X = 0x00FD`）。数值一致，但两处各写一份，改一处不会有任何失败。
- **不会 panic**（Python），但截断有两种坏结局，均已实测：
  - `zp` 短到 `$FD` 切片越界 ⇒ `int('',16)` 抛 `ValueError`，`align_case` 不接（`:180` 之外无 try）⇒ 整批崩。
  - `zp` 长度**奇数**（少 1 个 hex 字符，507）⇒ `zp[0xFD*2:0xFD*2+2]` 得到 1 个字符 `'a'`，`int('a',16)=10` —— **静默错值**，无任何告警。
- 无 `len(zp)==512` / 偶数长度 / 行内 `f` 存在性断言；`zp` 缺失行直接 `continue`（`:77-78`），可把整文件吞成空而不报错。
- 更稳的锚：要么从 `ram_map.rs` 机械导出常量（解析或生成），要么在本模块保留常量 + 启动时断言与 `ram_map.rs` 一致；至少 `assert len(zp)==512 and len(zp)%2==0` 并给出带行号的错误。

定谳：**必须改**。

### (c) `is_duplicate_second` 用文件字节判重 —— **实测可行，但契约不牢；建议改像素**

- `fc_align.py:95-98`：`a.read_bytes() == b.read_bytes()`。
- 实测（`case-p0c-r31`，f1720..1736 逐帧）：像素差为 0 的 4 对（1721/1726/1731/1736）**字节也相等**；其余 12 对字节与像素同时不等。即 Mesen2 当前编码链下「字节相等 ⇔ 像素相等」在本数据上成立，PNG 编码确定性没有反例。
- 但这是**编码器性质**，不是图像性质：换采集器（`bizhawk_shots.lua:44` 也产出 PNG）、后处理（optipng/diff 工具）、或改压缩级别，都会让「同图不同字节」→ 谓词假阴性 → 静默退回 seam（假失败方向，安全但会重现 ~700px 残影）。反向（不同图同字节）不可能。
- 另有一个更实的坑：比对对象 `emu_png[g-1]` 是 `_png(..., tag=None)` 选出来的（见 e），在稀疏/多 tag 目录里可能根本不是「视频帧 g-1 的显示面」。

定谳：当前可放过；**若保留字节比较，注释必须写明「依赖 Mesen2 编码确定性」，并在 `_png` 修好后才有意义**。改像素（numpy `array_equal`）成本极低，建议顺手做。

### (d) `diff_pixels` 重开图 / 批量性能 —— **PIL 不是瓶颈，`glob` 是；实测 62.8s/case**

- `diff_pixels`（`:133-141`）每行开两张图；单张 256×240，PIL 解码开销可忽略，且每行 g 基本不同，缓存收益小。
- 实测 `align_case('case-p0c-r31', 24 采点)` 全流程 **62.78s**（trace 0.00s / state 0.49s / 其余全在 `_png`）。
  根因：`:156-160` 对 **emu 轨迹每一帧**调一次 `_png`（1752 次目录 glob），`:163,:176` 每行再 glob 2~3 次；`shots/` 11864 文件、`shots_rust/` 52249 文件，实测单次 glob 6ms/29ms。
- 36 房 × 十几次采点的量级 ≈ 30+ 分钟，且全部浪费在重复扫描目录。
- 定谳：**性能问题真实存在，但根因不是 `diff_pixels`**；随 (e) 的「一次性建索引」一起解决即可，单独优化 PIL 无意义。

### (e) `_png` 的 glob + `hits[0]`：配错 tag —— **最严重缺陷，实锤**

- `_png`（`:125-130`）：`tag=None` 时模式是 `{case}_f{frame:05d}_*.png`，匹配**任意 tag**；`hits[0]` 取 `glob` 返回的第一个（`glob` 不排序，顺序依文件系统）。
- `align_case:163` 用它判断「该帧是采集点」，`:176` 又用它取比对图。于是 **`pre` / `prev{c}` / `prevend` 全部被当判定点**，而 `room_sweep.py:792-793` 明确 `if t == "pre" or t.startswith("prev"): continue  # prev* 只作 rust 参考帧，不参与判定`。
- 实测 `align_case('case-p0-0-02-end')`（7 行）：

  | c | 实际 tag | 结果 |
  |---|---|---|
  | 900 | `pre` | diff=0 → 计入 ok |
  | 929 | `prev30` | diff=0 → 计入 ok |
  | 959 | `prev60` | diff=0 → 计入 ok |
  | 1019 | `prevend` | diff=0 → 计入 ok |
  | 930 / 960 | `f30` / `f60`（**真判定点**） | `missing_png` |

  ⇒ `ok=4` **全部来自参考帧**，真实采集点一个都没算成。这不是「偶发配错」，是 `hits[0]` + 无 tag 约束的必然结果。
- 同帧多 tag 的碰撞是历史已知坑：`room_sweep.py:173-186` 的 `taken` 去重就是为它写的；`shots_rust/` 现存 **804** 个「同 case 同帧 ≥2 个非 diff PNG」的前缀（例：`_tmp_sweep_278073-1795412324_a2_r36_f01719_prev120.png` 与 `..._prevend.png`），`hits[0]` 在这些帧上等于抛硬币。
- 定谳：**必须改**。采集点身份应来自调度表（`captures` 的 `{frame,tag}`）或至少只认 `f{c}`；`hits` 必须排序、断言唯一；`prev*`/`pre`/`arrived`/`end` 显式拒绝。

### (f) 缺 `room_load` 时把载房窗判成 live —— **缺陷；有机械源**

- `choose_phase`（`:118-122`）只有 `room_load: bool` 入参；`align_case:174` 用 `c in load`；`load` 只来自 CLI `--room-load`（`:200,209`），无任何推导。默认空集 ⇒ 载房窗走 live。
- 实测：`c=1729 → g=1736` 被判 `live`；P1.0f 实测该点 live=**24818px**、seam=697px，且 24818 被定性为「活体已推进、显示未推进」的错帧（plan `:534-540`；`crates/game-core/src/bin/rust_screenshot.rs:562-576` 的载房窗分支正是为此存在）。即**默认参数会稳定产出 24818 假差**，且 operator 必须手工知道帧号。
- 机械源现成：`rust_screenshot.rs:607` 的 `--trace load` 已逐帧打印 `[load] f… phase={room_load_phase}`；且 `state_row(frame, mem)` 处的 `mem` 就带 `room_load_phase`（同文件 `:569,576`，`engine.mem.room_load_phase`）。把该字段塞进 `--state-out` 的 `w`（新增 `"rlp"`，与 zp 同缝），`align_case` 直接读 `rust_state[c].rlp` 即可，不需要 `--room-load`。
- 定谳：**必须改**（机械源优先；在拿到 rlp 之前，缺 `--room-load` 应 fail-loud，而不是默认 live）。

### (g) 其它（已核，非阻塞）

- `pair_frame` 的「唯一」在 `(FC,ScrollX)` 上**并非结构性成立**：实测 `c=1732`、key `(176,0)` → cands `[1739,1740]` 抛 `ValueError`。原因是非滚动段 `ScrollX=0` 使第二列失效，endFrame 轨迹在掉帧处重复 FC。方向是 fail-closed（安全），但会白丢点；且与 plan §A2「候选集结构性 ≤1」的论断不符（该论断只在 spin-exit/seam 表口径成立）。建议按 plan 的 `seam_table`（同 FC 取首现帧）实现后再断言唯一，或当两候选 PNG 像素相同时折叠。
- `diff_pixels` 尺寸不一致直接 `raise`（`:139-140`），在 `align_case:180` 无 try ⇒ 单点尺寸异常会崩掉整批；应记为该行 error。
- `parse_emu_trace` 缺列行静默跳过（`:58-59`），全空时 `main` 才报 `SystemExit`（`:206-207`）；`parse_rust_state` 无对应兜底（见 b）。
- `--live-dir` 强制必填（`:197`），但不校验它与 `--seam-dir` 是否同一 `.sched`/同一 case 的两次运行；跨运行混比会把输入差异记成内容差。属使用契约，建议在输出里带上两侧目录名与 sched 指纹。


## Q3 结论与处置

**verdict：需改后可用（仅诊断面；不得进 ratchet）。**

### 必须改的三条

1. **采集点身份 / tag 选择**（`fc_align.py:125-130,163,176`）
   只接受调度表里的判定点（至少只认 `f{c}`），拒绝 `pre`/`prev*`/`prevend`/`arrived`/`end`；glob 结果排序并断言唯一（对齐 `room_sweep.py:173-186,792-793` 的 `taken`/`prev*` 规则）。
   理由：实测一房 4/7 行、全部 `ok` 来自参考帧，当前 `ok` 计数不可信。

2. **相位输入契约与机械载房源**（`:118-122,156-160,174,200,209`）
   `emu_png` 从**目录索引**构建（一次扫描），不以 trace 键为集合——否则 seam 轨迹缺 `g-1` 行时 `is_duplicate_second` 恒 False、规则静默失效；断言配对窗内 emu 帧连续且 `g-1` 有图，否则显式 `phase_undecidable`（不得默认 seam 冒充结论）。载房窗改读 rust state 新增的 `room_load_phase`（源：`rust_screenshot.rs:607` / `state_row` 的 `mem`），缺该字段且无 `--room-load` 时 fail-loud。

3. **失败可见性与输入校验**（`:79-80,171-173,181-188,219-226`）
   `no_candidate`/`missing_png`/`多候选`/尺寸异常必须进 summary（如 `errors=`）并让退出码非零；配对上限取「`[c-h, c+h]` ∩ 可用 emu PNG ∩ trace」，并断言 `D=FC_rust(c)−FC_emu(c) ≥ 0`（否则说明单向窗假设被破）。同时断言 `len(zp)==512` 且偶数长度——实测 507 长度会静默算出 `$FD=10`。

### 可以放过

- **硬编码 `$2D/$FD`**：与 `ram_map.rs:50,66` 数值一致；建议加交叉断言或由 ram_map 导出，但不阻塞。
- **PNG 字节判重**：Mesen2 现链实测「字节相等 ⇔ 像素相等」无反例（1721/1726/1731/1736）；保留亦可，但需注明依赖，建议顺手改像素比较以抗重编码。
- **`diff_pixels` 重开图**：小图开销可忽略；实测瓶颈是 1752 次目录 `glob`（62.8s/24 点），随第 1/2 条的一次性索引消掉。
- **`horizon=40` 数值**：实测 max lag=8，余量 5×；改的是可见性与上限来源，不是数值。
- **多候选 `ValueError`**：方向正确（fail-closed）；重复帧候选图像像素相同时可折叠，属优化。


## 4. 复算命令（只读）

```bash
# 1) 采集点/tag 实锤（ok 全部来自参考帧）
（heredoc 片段按批次二/三略，实际命令见 fc_align 模块 docstring）
import sys; sys.path.insert(0,'.')
from tools.movie_ref import fc_align as F
from pathlib import Path
case='case-p0-0-02-end'
emu=F.parse_emu_trace(F.TRACES/f'{case}.frames.jsonl')
rust=F.parse_rust_state(Path('/tmp/p0/case-p0-0-02-end.state.jsonl'))
r=F.align_case(case,rust,emu,F.SHOTS_RUST,Path('/tmp/empty_live'),F.SHOTS)
print({k:v for k,v in r.items() if k!='rows'}); [print(x) for x in r['rows']]
（片段结束）
# 实测：frames=7 ok=4 nonzero=0 max=0；ok 的 4 行 c=900(pre)/929(prev30)/959(prev60)/1019(prevend)，
# 真采集点 c=930/960 为 missing_png。

# 2) 相位规则 / 多候选 / 缺 room_load
（heredoc 片段按批次二/三略，实际命令见 fc_align 模块 docstring）
import sys; sys.path.insert(0,'.')
from tools.movie_ref import fc_align as F
from pathlib import Path
case='case-p0c-r31'
emu=F.parse_emu_trace(F.TRACES/f'{case}.frames.jsonl')
rust=F.parse_rust_state(Path('/tmp/p0c.state.jsonl'))
r=F.align_case(case,rust,emu,F.SHOTS_RUST,Path('/tmp/empty_live'),F.SHOTS)
print({k:v for k,v in r.items() if k!='rows'}); [print(x) for x in r['rows']]
（片段结束）
# 实测：frames=24 ok=10 nonzero=0 max=0；c=1729 phase=live（无 room_load）；
# c=1732 多候选 [1739,1740]；c=1733..1740 missing_png；align 全流程 62.78s。

# 3) PNG 字节 vs 像素（Mesen2 编码确定性）
（heredoc 片段按批次二/三略，实际命令见 fc_align 模块 docstring）  ·3
import numpy as np; from PIL import Image
import os
for g in range(1721,1737):
    p=f'shots/case-p0c-r31_f{g:05d}_f{g-1600}.png'; q=f'shots/case-p0c-r31_f{g-1:05d}_f{g-1-1600}.png'
    a=np.asarray(Image.open(p).convert('RGB')); b=np.asarray(Image.open(q).convert('RGB'))
    print(g, int(np.any(a!=b,axis=2).sum()), open(p,'rb').read()==open(q,'rb').read())
（片段结束）  ·3

# 4) zp 截断静默错值
python3 -c "zp='a'*507; print(zp[0xFD*2:0xFD*2+2], int(zp[0xFD*2:0xFD*2+2],16))"
# -> 'a' 10
```

## 5. 未定谳 / 未验证

- 出样本相位规律（另一 mismatch 房、规则冻结后先跑）：**未做**，是 Q1 升格判据前的唯一硬缺口。
- `D<0` 的采点：现有 p0 数据未观察到，未定谳（单向窗目前不产生错判，只产生 no_candidate）。
- 36 房批量总耗时：按单房 62.8s 线性外推，未实跑（纪律禁止）。
- `--live-dir` 与 `--seam-dir` 同一 sched 的机械校验：代码中不存在，实际混比风险未量化。
