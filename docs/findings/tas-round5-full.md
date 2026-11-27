# TAS 100% 第五轮整片对拍：首分歧未离开 f1301（基线定谳）

> 日期：2026-11-27。任务 #133。写面：本文件（`tools/` 未改，`crates/` 零改动）。
> **前置结论（失败项前置）**：
> 1. 首分歧**仍在 f1301**（`$0454` offset84，emu `$00` / rust `$20`，2 字节）——卡面原判据「必须离开 f1301」**不成立**；
> 2. 门径载房窗**仍是 3 vs 2**（emu 冻结 3 帧 / rust 丢 2 帧），`$5A` 从 f1574 起差 1。
>
> 按 PMO seq4 的改后口径，本卡交付**基线定谳**：在含 #124/#129/#131 全部 pick 的 `84dcfe45` 上，把「未离开」钉成可复算读数，并给出「要离开它需要哪一条行为修复」。

## 1. 环境指纹与复现命令

- 工作树：`/home/peng/workspace/wt-133b`（分支 `lane/133b`，`git rev-parse HEAD` = `84dcfe45a1a4ec6bec2b9819ae6eb628f2fae91b`）
- `CARGO_TARGET_DIR=/home/peng/workspace/wt-133b-target`；Mesen2 单实例；refs 子模块 `d6f4d3f56`
- 整片：

```
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --segments 10000 --out /tmp/r133b/full \
  --allow-rust-missing --timeout 900
```

- 独立第二跑（覆盖 f1301 的等价口径，见 §5）：

```
python3 tools/tas_replay_diff.py replay \
  --inputlog movies/lordtom-metroid-100p.inputlog \
  --frames 10000 --segments 10000 --out /tmp/r133b/rerun10k \
  --allow-rust-missing --timeout 900
```

## 2. coverage.json 关键字段（整片）

| 字段 | 值 |
|---|---|
| `ran_frames` | **120540 / 120540** |
| 段数 / 失败段 | 13 / **0** |
| `coverage_pct` | 100.0 |
| `deterministic` | true |
| `first_divergence` | `frame 1301, field $0454, offset 84, emu 0, rust 32, diff_bytes 2` |
| `env.git_head` | `84dcfe45a1a4ec6bec2b9819ae6eb628f2fae91b` |
| `env.git_dirty_crates` | `[]` |
| `rust_anchor.anchor_sha` | `36d934eb1c4ba046` |

两侧产物 sha256（本机实算，`hashlib.sha256` 逐块）：

| 产物 | 字节 | sha256 |
|---|---|---|
| `rust/full.state.jsonl` | 2,472,687,630 | `a95d9c1fc02b8a086bc826851d549c11d91ce6d4bf500184e9bf7cc11f2cc633` |
| `coverage.json` | 10,615 | `6359173226f1ce4f7b56b0110b926dda12f71d19fe5193e570ea80dc73d95e9a` |
| 段0 emu 聚合 | — | `ce4031c36b4ecb78be008911659269e93a49b4fec7b6359f212406d24e89629f` |
| 段0 rust 聚合 | — | `cd70e896f3c12739cf6f8e14a0ff44a2999357ff88b9f9da048e2da1838ac65f` |

> 旁证：rust 帧表 sha 与 911cc6ae 那跑**逐字相同**（`a95d9c1f…`）——因为两次之间没有任何 `crates/` 行为提交；这也反证「未离开 f1301」不是本轮跑歪，而是行为面没动。

逐段（`from,to` / 判定 / 比较帧 / 分歧帧 / 该段首分歧）：

| # | 区间 | 判定 | 比较 | 分歧 | 段首分歧 |
|---|---|---|---|---|---|
| 0 | 0–10000 | diverge | 9932 | 8655 | **f1301 `$0454`** emu0/rust32（2B） |
| 1 | 10000–20000 | diverge | 10001 | 10001 | f10000 `$0400` 28/200（28B） |
| 2 | 20000–30000 | diverge | 9964 | 9964 | f20000 `$0400` 28/200 |
| 3 | 30000–40000 | diverge | 10001 | 10001 | f30000 `$0401` 78/31（25B） |
| 4 | 40000–50000 | diverge | 9965 | 9965 | f40000 `$0401` 78/31 |
| 5 | 50000–60000 | diverge | 10001 | 10001 | f50000 `$0400` 200/152 |
| 6 | 60000–70000 | diverge | 9955 | 9955 | f60000 `$0400` 200/152 |
| 7 | 70000–80000 | diverge | 9953 | 9953 | f70000 `$0400` 200/102（33B） |
| 8 | 80000–90000 | diverge | 9985 | 9985 | f80000 `$0400` 200/152 |
| 9 | 90000–100000 | diverge | 9955 | 9955 | f90000 `$0400` 120/152 |
| 10 | 100000–110000 | diverge | 9923 | 9923 | f100000 `$0400` 120/200 |
| 11 | 110000–120000 | diverge | 9964 | 9964 | f110000 `$0401` 213/212（20B） |
| 12 | 120000–120540 | diverge | 541 | 541 | f120000 `$0400` 165/168（17B） |

说明：段 1 起的「段首分歧」是**段起点**（已带首分歧后的漂移），不表示新的首个分歧；整片首分歧只有 f1301 一处。

## 3. 门径载房窗 f1571–f1577 逐帧对照（新采，非复用旧跑）

emu 侧为本轮在 `84dcfe45` 上新采的只读探针（`oracle` 场景自建，`watch_range 0000-00ff zp`，1600 帧）：
`analysis/traces/sc-133-window.frames.jsonl`；rust 侧取本轮 `rust/full.state.jsonl` 的 `w.zp` 同字节。

| f | emu `$2D` | emu `$5A` | rust `$2D` | rust `$5A` | 备注 |
|---|---|---|---|---|---|
| 1571 | 9 | 255 | 9 | 255 | 同 |
| 1572 | **9** | 24 | —（缺帧） | — | emu 冻结第 1 帧 |
| 1573 | **9** | 24 | —（缺帧） | — | emu 冻结第 2 帧 |
| 1574 | **9** | 241 | 10 | 240 | emu 冻结第 3 帧；rust `$5A` 差 1 |
| 1575 | 10 | 241 | 11 | 242 | 差 1 |
| 1576 | 11 | 243 | 12 | 244 | 差 1 |
| 1577 | 12 | 255 | 13 | 255 | 同 |

- emu `$2D` 冻结帧：**1572 / 1573 / 1574（3 帧）**；rust 缺帧：**1572 / 1573（2 帧）** ⇒ **3 vs 2 未变**。
- 窗口起点 f1571 两侧 `$56=$01, $58=$11, $5A=$FF, $2D=$09` 逐字节相同；到 f1577 两边都回 `$FF`。
- 机制口径沿用 #129 结论（`docs/findings/tas-round5.md`）：差异在**载房主循环迭代的起点相位**（跨 NMI 边界数），不是滚动调用次数（emu 窗内 1 次 / rust 0 次），也不是 `work_cycles` 总量少算（差 <1%）。
- 与 #132/#135 的接口：#132 给出 `dropped=(28454+work)/29780`、3 帧门槛 `work≥60886`、rust 实测 55308（缺口 5578）；#135 把这缺口里的 **1824** 定界为「属性区守卫触发两次（obj13 写 $63C0、obj14 写 $63DC 越过 $63A0），少画 2 full + 4 unique」。两者是**同一条门径窗链的两段**，但都**不影响 f1301**（f1301 早于 f1572，详见 §4 第 3 条）。

## 4. 要离开 f1301，需要哪一条行为修复进 `crates/`

**落点（文件/函数级，来自 #130 规格 `docs/spec/f1301-fix-plan.md`）**

- 文件：`crates/game-core/src/weapon/projectile.rs`
- 函数：Samus ↔ 敌/敌弹碰撞的**横向判盒**（约 `:529-541`，当前只比低字节 `dx`，阈值按 6px 判）
- 应有行为：按 ROM `07:F2CA` 之前的 16 位 X 轴比较（`lda hi / sbc` 那套）——即实现 `rom_axis_delta`：
  - 用 **hi 字节差**做主轴判定，且把 `sbc` 的**借位**计入 `dhi`；
  - hi 相等时才用低字节差；hi 不等即视为 ≥256px（不碰）；
  - `hi` 归一取 `PPUCTRL_ZP($00FF)` 的 bit0（nametable）；**不要**用 `$00FE`（那是 `PPUMASK`，bit0 是 greyscale，会让跨 nametable 场景静默失效——正是 f1301）。

**为什么这条能离开 f1301（A 级机制）**：f1301 时敌在 `EnsExtra.5.hi=0`（世界格内，x=120），子弹在 `Objects.13.hi=1`（下一张 nametable，x=126）；真实距离 **262px**，ROM 判「不碰」。Rust 只比低字节 6px ⇒ 误进 `07:F2CA` 写 `$040E`、`$0454`（`Ens.5.isHit` bit5）、`$045E`（`weaponAction`），并让子弹在 f1302 转 `status=4`。

**预期（可证伪，抄 #130 §6，不自造）**：修好后 `first` 不再是 f1301，且 f1301–f1349 的 `$0454/$045E/$0424/$042E/$0420/$0421/$0429/$042B/$042C/$042F` 一齐消失；次预测首分歧后移到 **f1551 `$040D`**（#122 记录的下一处地址，C 级，仅作参考）。

**先后关系（必须）**：f1301 < f1572 ⇒ **先落这条判盒修复**；门径窗（#129/#132/#135 链）另行处理。若先改门径窗，`replay` 的首分歧仍报 f1301，会把门径窗的效果掩盖掉。

## 5. 确定性

1. **独立第二跑**（同一棵树、同一命令形状，`--frames 10000` 覆盖 f1301）：
   `/tmp/r133b/rerun10k` → `ran_frames=10000`、`first_divergence` 与整片**逐字相同**（f1301 `$0454` offset84 emu0/rust32，2B）、`deterministic=true`、`git_head=84dcfe45a1a4`。
   口径差别：第二跑只到 10000 帧（整片 120540 的成本是 36 分钟/轮）；「首分歧逐字相同」所需的判据面在 f1301，已覆盖。整片那轮的**段内确定性**另由 `coverage.json.deterministic=true` 与 §2 产物 sha 记录。
2. **`--resume` 锚校验**：对 `/tmp/r133b/full` 重跑 `--resume` → 13/13 段全部「已完成（diverge，--resume 跳过）」，首分歧仍 f1301；锚 `anchor_sha=36d934eb1c4ba046` 绑定 `git_head=84dcfe45…` 与 rust 帧表 sha `a95d9c1f…`。旧 HEAD 的 `/tmp/r133/full`（`git_head=911cc6ae`）按设计**不能**被新 HEAD 复用（锚不匹配会拒绝）。

## 6. 门禁与回归

- `python3 gates/cli.py check` → **PASS=24 FAIL=0 SKIP=3 TOLERATED=1**
- `cargo test -q --workspace` → **1258 passed / 0 failed**（本卡干净树 `84dcfe45` 上 30 个测试二进制的 lib/bin 汇总；基线 1253 未破）。说明：PMO 报的 1259 是主检出**脏树**口径——主检出此刻带着 #125 未提交的测试，多 1 条；两边都不破基线。
- 边界：本卡**未改 `crates/`**、未改 `tools/`（探针场景文件放在 `/tmp` 与 `analysis/`，`analysis/` 不入库）；唯一入库改动是本文件。

## 7. 结论

在 `84dcfe45`（含 #124 更正、#129 计数取证、#131 typed 归位）上：
**首分歧没有离开 f1301**；门径窗仍 3 vs 2。原因不是「修复失效」，而是**这批 pick 里没有任何 `crates/` 行为提交**（#129 缩小为取证收口、#131 只做声明层）。
要离开 f1301，唯一需要落的是 §4 那条判盒修复（`weapon/projectile.rs` 的横向 16 位判盒 + hi 归一用 `$00FF`）。
