# oracle 验证链规格（三腿契约 + 仲裁规则）

> 状态：current（2026-11-18 首版）。本文回答一个问题：**当录像（TAS）、两个模拟器核心、Rust 重制
> 三方不一致时，谁有裁定权**。本文只定判据与路由，不重复各腿的操作细节（那些在
> `docs/findings/movie-bizhawk-6725m.md`、`docs/findings/movie-reference-core-fceux.md`、
> `docs/knowledge/gotcha-bizhawk-headless-mono-setup.md` 里）。

## 1. 三条腿各自证明什么

| 腿 | 实现 | 能证明 | 不能证明 |
|---|---|---|---|
| **A 路线腿** | BizHawk 2.10 + quickerNES（`tools/movie_ref/run_bizhawk_trace.sh`） | 某段代码在**真实通关路线**上被走到（覆盖证据） | 行为对错（该核与仪器腿本身有分歧，见 §3） |
| **B 仪器腿** | Mesen2（`tools/mesen2/`，与 L4 判据同源） | 逐帧 RAM 面、写点/执行回调、调色板精确像素 | 作者路线（同输入跑到与作者不同的地方） |
| **C 对照腿** | Rust 重制（`rust_screenshot --state-out`） | 移植是否与 B 一致 | 自己是否代表了 ROM（不能自证） |

## 2. 仲裁规则（本规格的核心条款）

1. **行为分歧的裁定权只在 B 腿**。A↔C 不一致而 B↔C 一致 ⇒ 判为**核心差**，不计入移植缺陷，
   登记到 §3 的噪声地板表。
2. **A 腿只出覆盖结论**：某房间/敌种/机制是否被真实路线触达。不得用 A↔C 的失配数当缺陷计数。
3. **禁止用输入平移消除分歧**。把输入整体移一格能让 Rust 拟合 A 腿的行为，那是拟合不是修复
   （2026-11-18 实例：+1 平移使 Rust↔A 达 1152/1152，而 Rust↔B 本来就已同帧对齐，见 §3 表）。
4. **跨腿帧号不直接比**。A/B 腿各自的记录相位必须先声明再比较：
   - A 腿记录器现为「先 `frameadvance` 再读」⇒ 行 f = frame f 的**收工态**（旧版是开工前态，
     两者相差一行，混读会产生系统性的「漂移」假象）。
   - B 腿 watch 为帧边界读；C 腿状态行 = tick `N-1` 收口（`crates/game-core/src/memmap/observe/frame_face.rs` 契约）。
5. **$0014 输入锁存面不得当帧轴判据**。它的最优点天然比状态面偏一行（两侧同属性）。
   2026-11-18 实测恒等式：**latch_offset == state_offset + 1**（biz post↔probe：状态 +1 / 锁存 +2；
   rust↔probe：状态 0 / 锁存 +1；pre↔probe 与 pre↔tmpaxis2 四组配对一致）。跨腿 `offsets` 必须取
   状态面（status/room 共识）；`$0014` 单列 `latch_offsets`。反例：把 +2/+1 当帧偏移会让 biz 多 8 个、
   rust 多 53 个失配帧（2026-11-18 Task 1 首次交付，已被复核 reject，见 `docs/rounds/r9-task-1-review.md`）。

6. **帧偏移分两面求，且只有一面允许评分**：
   - **状态面（唯一产出 `offsets` 的面）**：用 `$0300 SamusStatus` 与 `$005A RoomNumber` 的**全帧一致率**取
     唯一最大（这是本规格唯一允许"评分选优"的场合；两个字段必须给出同一 k，否则 fail-loud）。
     实测 `data/status/leg_offsets.json`：`biz_to_mesen=+1`（status 1096/1180、room 1158/1180）、
     `rust_to_mesen=0`（status 1154/1166、room 1158/1166）。
   - **锁存面（只用于校验，不得产出帧偏移）**：用 `$0014` 的**值变化帧**做唯一反解
     （`change_runs < 3` 或多解即 fail-loud，不许挑一个），其结果只用来验 R5 的恒等式
     `latch_offset == state_offset + 1`（实测 `+2`/`+1`）。锚残差（全帧口径）biz 10/1179、rust 6/1166，
     集中在 f661..673 与 f1132..1133（后者仅 biz，是 p1→latch 前向滞后 2 帧；rust 残差止于 672）。
   - **锁存面异常（gap≠+1 或缺键）不得否决状态面结果**：此时 `latch_offsets=null`、
     `latch_gap_anomaly/latch_missing=true`、exit≠0，但 `offsets`/`per_field` 必须保留真实值
     （2026-11-18 Task 1 第二轮复核点名的误杀形态）。

7. **加载窗帧不进分母，但必须显式列账**。`$005A RoomNumber != $FF` 的帧是房间加载瞬态
   （`constants_ram.asm:589`：`#$FF=no room requested`），此时三腿的加载链进度不同，比较无意义。
   报告必须同时给出「总帧 / 进分母帧 / 排除帧及其原因」，排除帧逐帧列表落盘，禁止只报一个分母数字。
   （"此时比较无意义"是推测，未做单测；已证实的是：该窗内三腿 `$005A` 取值集合并不相同。）
   实测（`data/status/leg_episodes.json`）：窗口 1180 帧，排除加载窗 33 帧（其中含 14 帧对侧缺失），
   进分母 1147；`by_leg` biz 1147 / rust 1155。

## 3. 噪声地板（A 腿不可用于归因的量化依据）

同一 `movies/eien86-6725M.inputlog`、窗口 f40..1200、四字段 `$0106/$0107/$0300/$005A`：

| 比较 | 最优相位 | 四字段全同 |
|---|---|---|
| Rust ↔ Mesen2 | f↔f | 1133/1153（98.3%） |
| quickerNES ↔ Mesen2 | f↔f | 1061/1160（91.5%） |
| Rust ↔ quickerNES | f↔f | 1062/1153 |

⇒ **同期逐帧字段相同率约 91%，但把失配按「段」分类后，真正的行为分歧只有 2 段**：见 §3.3。
读数口径必须先说清：上表的正确相位映射是 **`post[f]` ↔ `emu[f+1]`**（见 §3.1），换错映射会产生一整片
并不存在的「分叉」。

### 3.1 记录器相位改名不是物理位移（全片实测）

把 A 腿记录器从「开工前读」改为「先 `frameadvance` 再读」后，用全片 28408 帧对照：

| 比较 | 结果 |
|---|---|
| `post[f]` == `pre[f-1]` | **28388/28388 = 100%** |
| `post[f]` == `pre[f]` | 26953/28388 = 95.0% |

⇒ 两者是**同一物理相位的重新编号**（新文件行 f 装的是旧文件行 f+1）。因此：

1. 换相位**不会**改变任何行为结论（`rust[f]` ↔ `post[f-1]` 与 `rust[f]` ↔ `pre[f]` 同值）；
2. 但**新旧文件绝不能混读**：混读会凭空产生「整体漂移一行」的假象——这正是 2026-11-18 那次
   「输入 +1」误判的形态。

#### 3.1.1 帧映射规约（2026-11-18 实测）

- `tools/movie_ref/align_legs.py` 的跨腿帧偏移由**状态面**（`$0300` SamusStatus + `$005A` RoomNumber）
  的**全帧一致数**反解；产物 `data/status/leg_offsets.json`。状态面大部分帧不变，只取变化帧样本太少。
- `offsets` = 状态面共识；`latch_offsets` = `$0014` 锁存面单列。实测
  `latch_offset = state_offset + 1`（biz +1/+2，rust 0/+1，四组配对），因此锁存面只作 ±1
  corroboration，不得单独产出整数帧偏移。
- `change_frames` 与 `change_runs` 双计数；`change_runs < 3` 视为不可对齐。`offsets` 只用于状态面，
  `$0014` 只用于 `latch_offsets`。
- 锁存面是**佐证面，不许一票否决状态面**：`latch != state + 1` 时仍保留 `offsets`/`per_field`，
  只把 `latch_offset` 置 null、`latch_gap_anomaly=true`，verdict 仍可报
  `latch-state-gap-anomaly` 并退出 1；Mesen2 行缺 `Joy1Latch` 时 `load_state` 的 `latch` 为 null
  （`load_latch` 仍 fail-loud），CLI 保留状态面并报 `latch-unavailable`。
- 状态面字段一致率并列时报 `ambiguous-offset`；`status`/`room` 各自最优 offset 不一致才是
  `field-disagreement`。
- JSON 的 `denominators` 小节显式记录三种口径：`anchors` = 变化帧（`change_frames`/`change_runs`），
  `per_field` = 全帧，`residuals` = 全帧（锁存残差 biz 10/1179、rust 6/1166）。
- 事故登记：2026-11-18 Task 1 首次交付把锁存面 +2/+1 写进 `offsets`，独立复核判 reject（P0）。
  代价是 biz 多 8 个、rust 多 53 个失配帧；Task 2 的 phase/value 分类会被假相位淹掉。本节即更正。

### 3.2 全片一致率只有 6.4%（级联，不是 28295 个缺陷）

同输入全片五字段（`health_lo/health_hi/status/room/scrolldir`）一致率：**1809/28295 = 6.39%**。
这不是 2.8 万个移植缺陷——首个实质分歧（按 Mesen2 口径 f1131 之前）之后两侧走到**不同的房间/路线**，
此后逐帧比对失义。判读纪律：**分歧点之后的一律不计入缺陷计数**，只看首个分歧点及其前窗。

### 3.3 失配按段分类：粗判 2 段真差，其中 1 段落在加载窗（2026-11-18 实测，经 §3.4 更正）

把 f20..1199 窗口按「连续失配段」聚合，再判每段是否只是 ±1 帧相位（判据：该段起点的值能在对面
±2 帧内找到同值）：

| 段 | 长度 | 字段 | 判定 |
|---|---|---|---|
| f23..28 | 6 | room/status/hh/sd | 相位（上电/入场） |
| f671 / f674..677 | 1+4 | room | 相位 |
| **f943..952** | **10** | **room + status** | **真差** |
| **f1030..1080** | **51** | **status（A 记 Roll 停滞）** | **真差** |
| f1092..1194 | 各 1 帧 | status | 相位（`iter_span` 抖动） |
| f1130..1135 | 6 | room/status | 相位 |

> 【2026-11-18 更正】本表按「段起点」粗判。经 §3.4 的整段序列匹配复核：`f943..952` 是**加载窗**
> （`during_load=true`，落在加载窗内），不再计为干净行为分叉；`f1030..1080` 在更严判据下
> **仍为真差**（唯一干净窗内的持续 `value` 段）。表内「真差」二字请按 §3.4 读。

⇒ 结论（按 §3.4 更正后）：A 腿与 B 腿在**干净窗内**的实质分歧只剩 **f1030..1080 status** 一段，
而不是「8.5% 的帧」；`f943..952` 落在加载窗内，按 R7 不进归因分母。
1 帧抖动段一律不计入缺陷，也不得用来支撑「核不可用」的结论。**首批可归因样本 = f1030..1080 的退球机制。**

### 3.4 三分类 episode 实测（2026-11-18，覆盖并修正 §3.3 的粗判）

§3.3 的判据是「段起点的值能否在对面 ±2 帧内找到同值」，它会把「一帧稀有值」放大成整段真差。改用
**整段序列平移匹配**后（`tools/movie_ref/classify_episodes.py`，产物 `data/status/leg_episodes.json`）：

| 类 | 判据 | 实测 |
|---|---|---|
| `phase` | 存在 k∈[-2,2] 使本腿整段值序列 == 对面平移 k 后序列（k 记入 `evidence.shift`） | 6 段 |
| `transient` | 段 ≤2 帧，且对面 ±2 内有 1 帧不同值尖峰 | 31 段（rust 4 + biz 27 个单帧 status 尖峰） |
| `value` | 无任何 k 命中，且分歧持续 ≥3 帧或该值在对面 ±5 内完全不存在 | 6 段 |
| `undecidable` | 窗内两侧各自恒定且不等（常量字段），不默认 phase | 0 段（分支由合成测试覆盖） |

**区间轴注意**：episode 的 `start/end` 记在 **B（Mesen2）轴**上（比较时 biz 取 `biz[f-1]`，见
`tools/movie_ref/classify_episodes.py` 的 `classify`），所以 "mesen f1031..1081" 对应 **biz 行 1030..1080**，
与 §3.3 的 `f1030..1080` 是同一段。

关键读法：**6 个 `value` 段里 5 段 `during_load=true`**，唯一干净窗内的持续值差是
**mesen 轴 f1031..1081 status**（biz 行 1030..1080，51 帧）——即 §3.3 的「f1030..1080 退球」结论在更严
判据下依然成立。`f943..952` 落在加载窗内（`during_load=true`，biz room 出现 242 而 mesen 全程无 242），
按 R7 **不计入归因分母；成因未定**（"是加载动画序列不同"只是未单测的假设）。
W 由实测**前向**滞后分布导出（`p1 变化 → 其后首个 latch 变化`：0 帧 96 次、2 帧 1 次，
`max(forward)=2` ⇒ W=2）；最近邻口径给出 `max_abs=1` 与 `-1` 一例，仅作诊断、不参与推导；
W 敏感性实测 W=2/3/4 → phase 6/18/37、transient 31/15/0、undecidable 0/4/0，**W=4 会让 transient
类整体消失**，故不采用。`nearest_equal_distance.q95=4` 只是诊断量，不参与 W 推导。

## 4. 已排除的腿（负结果，避免重复投入）

| 候选 | 结论 | 证据 |
|---|---|---|
| FCEUX 0.98.16（100% 录像声明的核心） | 不可得：2008 时代源码不可取；2009 树可编但加载 movie 即 `FCEUD_VideoChanged/Quit` | `docs/findings/movie-reference-core-fceux.md` |
| FCEUX 2.2.2 代跑 100% 录像 | **版本不对应制造伪分歧**：Samus 活到 f4300（hp `$0140`）而 Mesen2 在 f3890 判死 | 同上 |
| BizHawk NesHawk 作仪器腿 | **回调可用、路线不可用**：`System Bus` 域写/执行回调真实命中（60 帧 1155 次），但同 inputlog 300 帧停在标题画面（quickerNES 已进 Brinstar）；上电填充对齐与四种输入对照组均不解 | 2026-11-18 探针（`/tmp/bhprobe/`，非入库） |
| BizHawk quickerNES 的 Lua 内存回调 | **通道为零**：RAM 与 `System Bus` 域注册均返回空 GUID、0 命中（同帧 RAM 差分证明确有 1807 字节变化） | `docs/findings/room-5a-transition-verdict.md` §2 |

## 5. 产物与标注约定

- 任何来自 A 腿的产物必须在文件名或表头带 `biz-bwhawk`/`bizhawk` 字样；来自 B 腿带 `mesen`；
  C 腿带 `rust`。混用目录时以 `leg:` 字段标注。
- 轨迹文件表头必须写明**记录相位**（收工态/开工前态/帧边界读），缺相位声明的轨迹视为不可用。
- A 腿轨迹的 CSV/JSONL 若由旧记录器生成，必须在同一目录留一行 `# phase=pre-work` 说明，禁止与新
  口径文件同名混读。

## 6. 复现命令（最小集）

```
# A 腿：全片轨迹（约 8 分钟，exit=137 属正常，判据是行数）
bash tools/movie_ref/run_bizhawk_trace.sh "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" \
  28408 analysis/traces/bizhawk-6725M-post.jsonl

# B 腿：同输入的逐帧 RAM（watch 全 RAM）
python3 tools/l4_diff_run.py run <case 文件>

# C 腿：Rust 状态行
cargo run -q --release -p game-core --bin rust_screenshot -- --state-out /tmp/rust-states.jsonl ...

# 三方对比（每个相位都算，取最优并打印全部）
python3 tools/movie_ref/compare_bizhawk_rust.py --bizhawk <A> --rust <C> --inputlog <inputlog>
```

## 7. 未决项

1. quickerNES 与 Mesen2 那 8.5% 的**性质**未定（是核的早期行为分歧，还是 A 腿记录器仍有相位残差）。
   处理原则：逐段在 B 腿复核，复核不清的段落不得用于归因。
2. NesHawk 「进不去游戏」的机制未定论（可能是 harness 的 joypad 注入对它是空操作，也可能是核的
   PPU/APU/mapper 早期行为分歧）。若要复活该腿，先做「PC 执行覆盖对比」。
3. A 腿的 `.bk2` 跨核加载不可用（换核心后卡模态对话框，150 s 超时 0 行）。
