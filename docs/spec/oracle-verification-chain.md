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
5. **$0014 输入锁存面不得当帧轴判据**。它的最优点天然比状态面偏一行（两侧同属性），拿它对齐会
   假阳性（2026-11-18 实测：Rust↔Mesen2 锁存最优 f↔f+1 1147/1153，而状态面最优 f↔f 1133/1153）。

## 3. 噪声地板（A 腿不可用于归因的量化依据）

同一 `movies/eien86-6725M.inputlog`、窗口 f40..1200、四字段 `$0106/$0107/$0300/$005A`：

| 比较 | 最优相位 | 四字段全同 |
|---|---|---|
| Rust ↔ Mesen2 | f↔f | 1133/1153（98.3%） |
| quickerNES ↔ Mesen2 | f↔f | 1061/1160（91.5%） |
| Rust ↔ quickerNES | f↔f | 1062/1153 |

⇒ **quickerNES 与 Mesen2 在约 8.5% 的帧上字段不同**。这个量级大于我们要判的多数分歧，所以 A 腿的
失配必须先在 B 腿复核才能升级成缺陷卡。首批实例：`status` 在 f1031..1080（A 记 `01`、B 与 C 同为 `03`）。

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
