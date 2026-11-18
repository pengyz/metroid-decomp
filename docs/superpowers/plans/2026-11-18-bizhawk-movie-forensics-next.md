# BizHawk 录像取证 → 全片轨迹与 room 过渡定谳 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把「作者核心轨迹」从 1201 帧补到全片 28408 帧，并定谳前 1000 帧里唯一的真实行为分歧（`$5A` 门过渡）——给出 A 级证据的裁定，而不是再多的观测。

**Architecture:** 五条腿互相独立、各自可验证：① BizHawk 轨迹拉长（纯取证）；② inputlog 按已修复的 `movie_import` 重生成（产物收敛）；③ `$5A` 写序列真值（模拟器侧插桩）；④ Rust 写点静态对照 + 定性裁定（不给结论不罢休）；⑤ 跨模拟器互证（quickerNES ↔ Mesen2，判定既有 Mesen2 结论能否迁移）。前四条只产生**证据**，不改游戏逻辑；改逻辑在「不在本计划」段，需单独批准。

**Tech Stack:** BizHawk 2.10 + mono 6.8（quickerNES 核心，无头配方已入库）、Mesen2 2.1.1（`tools/scenario_run.py` DSL + `inputlog` 指令）、Rust `rust_screenshot`（`--schedule` / `--state-out`）、Python 3 比对工具（`tools/movie_ref/compare_bizhawk_rust.py`）。

**Spec:** `docs/findings/movie-bizhawk-6725m.md`（本计划的全部数字与地址锚来源；执行者必读 §4 地址表、§6.1 比对协议、§6.4 过渡表、§7 缺陷登记）。

## Global Constraints

- **RAM 地址只从仓库锚取**：`$0106/$0107` = `crates/game-core/src/ram_map.rs:169`；`$0300` = `:223`；`$005A` = `:216`；`$0049` = `:59`；`$0014/$0015` = `:70,:73`。新地址必须先补锚再使用，禁止猜。
- **帧轴协议不变**：Rust 状态行 `f=N` ↔ 轨迹行 `f=N`（Rust 帧 N 消费录像第 N-1 帧输入）。上电窗口 `f<20` 不计入比对（quickerNES 上电填 `0xFF` vs Rust `0x00`）。
- **缺行语义不变**：Rust 侧无状态行 = `iter_span_left>0` 的「无采样缝」（`crates/game-core/src/frame/engine.rs:112-126`），**不是崩溃**；判据是 stderr 无 `引爆缺口`。
- **`analysis/` 不入库**、`movies/` 已 gitignore、`rom/` 永不改、门禁不弱化/不调阈值、`cases/` 不留临时用例。
- **生产逻辑禁止 `std::env` 钩子**（AGENTS.md §3 规则 8；门禁 `no-env-hooks`）。需要新观测量时走 `src/bin/` 启动参数或独立测试。
- **每次 commit 前跑** `python3 gates/cli.py check`（快层）；提交信息按 `type(scope): subject` + 正文 + `Evidence:` + `Confidence:`。触及 `data/status/baselines/` 必须带 Evidence。
- **并发纪律**：EmuHawk / Xvfb **串行**跑（共用 `/tmp/bh/config.ini` 与日志）；两个模拟器腿不得同时写同一产物目录。工作区已有他人在写未跟踪文件（`tools/movie_ref/compare_bizhawk_rust.py`、`movie_sched.py`），**只读复用、不并入自己的 commit**。
- **无头配方唯一入口**：`tools/movie_ref/run_bizhawk_trace.sh`（`ALSOFT_DRIVERS=null` + `SDL_AUDIODRIVER=dummy` + `xvfb-run` + 绝对路径化；判据=产物行数，退出码 137 是 BizHawk 自杀，不是失败）。

## Review Focus

1. **轨迹提前结束却看起来「跑完了」**：录像在某帧后无输入/状态分支变化时，BizHawk 仍会逐帧前进，但若 ROM 侧复位或 Lua 抛错提前 `client.exit()`，产物会短一截，而 `run_bizhawk_trace.sh` 的 `PARTIAL` 只看行数不看到底停在哪 ⇒ 必须断言**末行 `f` 值**等于目标帧数，而不只是行数。
2. **末行半截**：轨迹与 Rust 状态行都可能以半行结束（写入中读取）；`compare_bizhawk_rust.py` 容忍末行，但"可比帧"分母会因此漂移 ⇒ 比对前必须确认两侧进程已退出、末行是完整 JSON。
3. **inputlog 位序修好后仍有第二层错**：`|` 是分组符、`LogKey` 的 `|` 是列分隔符，只改 `bits_from_text` 的列序而不改分组解析会「看起来对了」⇒ 必须有 f228=`0x02`(Left) 与 f13=`0x10`(Start) 的**逐值断言**，不能只看行数。
4. **写回调只覆盖 CPU 写**：`event.onmemorywrite` 抓的是 CPU store；若 `$005A` 由其它路径（间接写、被 NMI 覆写）改动，回调会漏 ⇒ 必须与「逐帧读值」两侧交叉（写序列能重建出读序列才算抓到）。
5. **并发 EmuHawk 产生幻影行**：两个进程共用 `/tmp/bh` 配置/日志与同一 Xvfb 显示时会互相污染（历史 `gotcha-concurrent-mesen-output-corruption` 同族）⇒ 跑前 `pgrep -af 'EmuHawk|Xvfb'` 必空，产物写唯一路径。

---

### Task 1: 全片轨迹导出（28409 行）+ 确定性前缀校验

**Files（Task 1）:**
- Create: `analysis/traces/bizhawk-6725M-full.jsonl`（不入库）
- Create: `analysis/traces/bizhawk-6725M-full.sha256`（不入库，供复算）
- Read: `tools/movie_ref/run_bizhawk_trace.sh`、`analysis/traces/bizhawk-6725M.jsonl`（1201 行基准）

**Interfaces（Task 1）:**
- Consumes: 无（独立）
- Produces: `bizhawk-6725M-full.jsonl`，行 `{"f":0..28408,...}`，字段集与 1201 行版完全一致（`f/health_lo/health_hi/status/room/scrolldir/p1/p2/joy1_latch/joy2_latch`）。后续 Task 4/5 与任何「1000 帧之后」的对拍都消费它。

- [ ] **Step 1: 确认无并发进程**

```bash
pgrep -af 'EmuHawk|Xvfb' || echo "clean"
```
Expected: 打印 `clean`（若列出进程，等它退出，不要杀别人的）。

- [ ] **Step 2: 跑全片导出（约 30 min，后台跑）**

```bash
cd /home/peng/workspace/metroid-decomp
TIMEOUT=2400 tools/movie_ref/run_bizhawk_trace.sh \
  "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" 28408 \
  analysis/traces/bizhawk-6725M-full.jsonl
```
Expected: `输出行数 : 28409`、`STATUS: OK`；退出码 137（预期）。

- [ ] **Step 3: 断言末帧与行数（不是只看行数）**

```bash
tail -1 analysis/traces/bizhawk-6725M-full.jsonl | python3 -c 'import json,sys; r=json.load(sys.stdin); assert r["f"]==28408, r["f"]; print("last f OK:", r["f"])'
wc -l < analysis/traces/bizhawk-6725M-full.jsonl
```
Expected: `last f OK: 28408` 与 `28409`。若 `f` 小于 28408 ⇒ 录像提前结束或 Lua 中途退出，**不得**标记完成，先查 `--chromeless` 日志里 Lua 的 `wrote N rows` 行。

- [ ] **Step 4: 确定性前缀校验（免费拿到「两次运行逐字节相同」的强证据）**

```bash
head -1201 analysis/traces/bizhawk-6725M-full.jsonl | diff -q - analysis/traces/bizhawk-6725M.jsonl && echo "prefix identical"
```
Expected: `prefix identical`。不一致 ⇒ 两次运行不同源（并发污染或核心随机性），停止并记 findings 假设区。

- [ ] **Step 5: 落 sha256 并记录成本**

```bash
sha256sum analysis/traces/bizhawk-6725M-full.jsonl | tee analysis/traces/bizhawk-6725M-full.sha256
```
Expected: 一行 64 hex + 路径；同时记下 wall 时间（用于后续估算）。

- [ ] **Step 6: 更新 findings §5（轨迹产物）与 §9（未做清单）**

在 `docs/findings/movie-bizhawk-6725m.md` §5 追加「全片段」：28409 行、末帧 28408、sha256、前缀一致性结论、wall 时间；§9 删除「全片未导出」一项。
不写新结论，只登记事实。

- [ ] **Step 7: Commit**

```bash
git add docs/findings/movie-bizhawk-6725m.md
git commit -m "docs(findings): 6725M 全片轨迹导出（28409 行/末帧 28408）+ 前缀确定性校验"
```

---

### Task 2: inputlog 按修复后 `movie_import` 重生成 + 位序逐值断言

**Files（Task 2）:**
- Modify: `movies/eien86-6725M.inputlog`（gitignored，重生成）
- Modify: `movies/eien86-6725M.provenance.json`（gitignored，重生成）
- Read: `tools/movie_import.py`（:88-160 `parse_bk2` + LogKey 解析）、`tools/tests/test_movie_import.py`

**Interfaces（Task 2）:**
- Consumes: `77302b66` 的 `parse_bk2`（读 `LogKey` 建列描述表）。
- Produces: 修正位序后的 `.inputlog`（每行 `p1 p2 cmd` hex，UI 位形 A=bit7…Right=bit0）。Task 5 与所有 Mesen2/`scenario_run.py` 的 `inputlog` 用例消费它。

- [ ] **Step 1: 备份旧产物并重生成到新路径（先不覆盖）**

```bash
cd /home/peng/workspace/metroid-decomp
cp movies/eien86-6725M.inputlog /tmp/inputlog-old.txt
python3 tools/movie_import.py convert "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" \
  -o /tmp/inputlog-new.txt --frames 1200
wc -l /tmp/inputlog-old.txt /tmp/inputlog-new.txt
```
Expected: 两侧均为 1200 行（`--frames 1200` 截齐，便于逐行 diff）。

- [ ] **Step 2: 逐值断言（Review Focus 3——不能只看行数）**

```bash
sed -n '229p;14p' /tmp/inputlog-new.txt
```
Expected: 第 229 行首字段 `02`（录像 f228 = `P1 Left`）、第 14 行首字段 `10`（录像 f13 = `P1 Start`）。
若不符 ⇒ 立刻停：要么 LogKey 解析仍有错，要么帧基不同（f0 vs f1），记 findings 并回 `tools/tests/test_movie_import.py` 定位。

- [ ] **Step 3: 统计新旧差异，确认「只有方向键相关行变化」**

```bash
diff <(cut -d' ' -f1 /tmp/inputlog-old.txt) <(cut -d' ' -f1 /tmp/inputlog-new.txt) | grep -c '^[<>]'
python3 - <<'PY1'
old=[l.split()[0] for l in open('/tmp/inputlog-old.txt')]
new=[l.split()[0] for l in open('/tmp/inputlog-new.txt')]
import collections
c=collections.Counter((int(o,16)^int(n,16)) for o,n in zip(old,new))
print("xor histogram:", dict(sorted(c.items())))
PY1
```
Expected: 非零 xor 只落在 `0x01/0x02/0x04/0x08` 的位（方向键错位），**不出现** `0x10/0x20/0x40/0x80`（Start/Select/B/A 位置本来就对）。出现后者 ⇒ 修复引入新错位，停。

- [ ] **Step 4: 覆盖产物并重生成 provenance**

```bash
python3 tools/movie_import.py convert "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" \
  -o movies/eien86-6725M.inputlog --frames 1200
python3 tools/movie_import.py inspect "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" | head -20
```
Expected: `inspect` 的 `frames_with_input`/`first_p1_frame` 与 findings §1 一致（19415 / 13）。

- [ ] **Step 5: 用新 inputlog 跑 Rust 腿 + 比对，确认与 findings 的 10 帧失配一致**

```bash
python3 tools/movie_ref/movie_sched.py --inputlog movies/eien86-6725M.inputlog \
  --frames 1200 --out /tmp/sched-fixed.sched
cargo run -q -p game-core --bin rust_screenshot -- --schedule /tmp/sched-fixed.sched \
  --out /tmp/rustshots-fixed --state-out /tmp/rust-states-fixed.jsonl
python3 tools/movie_ref/compare_bizhawk_rust.py \
  --bizhawk analysis/traces/bizhawk-6725M.jsonl --rust /tmp/rust-states-fixed.jsonl \
  --inputlog movies/eien86-6725M.inputlog --fail-on-mismatch
```
Expected: 失配集合 = findings §6.2 的 10 帧 `{27,28,29,940,944,945,948,949,950,952,953}`（`--fail-on-mismatch` 会以非零退出，这是**预期**：把非零当"与基线一致"的证据，不是失败）。**关键断言**：输入面 `joy1_latch` 失配数必须回到 **0/967**（旧 inputlog 时是 f229 起错位）。
若 `movie_sched.py` 尚未入库（他人在写），改用 findings §8 的内联展开命令，并在 commit 里注明未用该工具。

- [ ] **Step 6: Commit（只提交文档/工具，不提交 movies/）**

```bash
git add docs/findings/movie-bizhawk-6725m.md
git commit -m "docs(findings): inputlog 按修复后 movie_import 重生成——位序锚 f228/f13 逐值复核，输入面 0/967 失配"
```
正文写明：旧产物错位映射、xor 直方图结论、修后与 findings §6 的 10 帧失配一致。

---

### Task 3: `$005A` 写序列真值（模拟器侧插桩）

**Files（Task 3）:**
- Create: `tools/movie_ref/bizhawk_writes_5a.lua`（新脚本：只记 `$005A` 写事件）
- Create: `analysis/traces/writes-5a-6725M.jsonl`（不入库）
- Read: `crates/game-core/src/ram_map.rs:216`、findings §6.4

**Interfaces（Task 3）:**
- Consumes: Task 1 的结论（可以先跑短窗口 900..1000 试通，再上长窗口）。
- Produces: 每行 `{"f":N,"value":V,"pc":P,"seq":k}` 的写序列；`seq` 是 `$005A` 的**单调写序号**，用于跨模拟器/跨实现对齐（不依赖帧内顺序）。Task 4 消费它。

- [ ] **Step 1: 探针——确认 `event.onmemorywrite` 在本构建可用且签名**

写 `/tmp/probe_writes.lua`：
```lua
local n = 0
event.onmemorywrite(function(addr, value, pc)
  if addr == 0x005A then
    n = n + 1
    console.log(string.format("W5A f=%d v=%02X pc=%04X", emu.framecount(), value, pc))
  end
end)
for i = 1, 60 do emu.frameadvance() end
console.log("probe total=" .. n)
client.exit()
```
跑 60 帧（`BH_FRAMES=60` 的配方 + 该探针），检查是否打印 `W5A` 或 `probe total=`。
Expected: 打印 `probe total=N`。若报错（签名不同）⇒ 依次试 `event.onmemorywrite(fn, 0x005A)`、`memory.registerwrite(0x005A, fn)`、`memory.registerexec`，把**可用的那一个**记进脚本头注释（这就是 A 级 API 证据）。

- [ ] **Step 2: 写 `tools/movie_ref/bizhawk_writes_5a.lua`（沿用探头验证过的签名）**

要点：只在 `addr==0x005A` 时写一行 JSON；`seq` 递增；`flush` 每 100 行；输出路径读 `BH_OUT`（与 `bizhawk_ref.lua` 同约定：默认 `analysis/traces/writes-5a-6725M.jsonl`）；帧数读 `BH_FRAMES`。

- [ ] **Step 3: 跑窗口 f=0..1100（覆盖 944-951 且留上下文）**

```bash
ALSOFT_DRIVERS=null SDL_AUDIODRIVER=dummy BH_FRAMES=1100 \
  TIMEOUT=300 tools/movie_ref/run_bizhawk_trace.sh \
  "/tmp/m6725/eien86,alyosha,the8bitbeast-metroid.bk2" 1100 /tmp/writes5a.jsonl
```
（若脚本只认 `bizhawk_ref.lua`，则用 Task 3 Step 1 的同一命令行形式手动跑该 lua。）
Expected: 有产物且非空。

- [ ] **Step 4: 自校验——写序列必须能重建逐帧读序列（Review Focus 4）**

```bash
python3 - <<'PY2'
import json
writes=[json.loads(l) for l in open('/tmp/writes5a.jsonl')]
traj={json.loads(l)['f']: json.loads(l)['room'] for l in open('analysis/traces/bizhawk-6725M.jsonl')}
cur=None; bad=[]
for f in sorted(traj):
    for w in [w for w in writes if w['f']==f]:
        cur=w['value']
    if cur is not None and traj[f]!=cur: bad.append((f,traj[f],cur))
print("writes:",len(writes),"mismatch frames:",bad[:8],"n=",len(bad))
PY2
```
Expected: `mismatch frames: []`（写序列重建出的值 == 轨迹读值）。非空 ⇒ 有写没抓到（漏路径），在 findings 记「观测面缺口」，Task 4 的裁定降级为「基于不完整写序列」。

- [ ] **Step 5: 抽 f=938..955 的写序列，回答「`09` 是否被写入过」**

```bash
python3 - <<'PY3'
import json
for l in open('/tmp/writes5a.jsonl'):
    r=json.loads(l)
    if 935 <= r['f'] <= 958: print(r)
PY3
```
Expected: 要么出现 `value=9`（⇒ Rust 缺的是**写入**，属行为分歧），要么不出现（⇒ `09` 来自 f944 之前某帧的**残留/覆写时序**，需看 pc 归属）。把结论连同原始行贴进 findings。

- [ ] **Step 6: Commit（脚本 + findings 增补）**

```bash
git add tools/movie_ref/bizhawk_writes_5a.lua docs/findings/movie-bizhawk-6725m.md
git commit -m "feat(tools): $005A 写序列取证 Lua（event.onmemorywrite 探针实证）+ 6725M f935..958 写序列"
```
正文附：探针实际可用的 API 形式、`mismatch=[]` 自校验、写序列原始行。

---

### Task 4: Rust 侧 `$005A` 写点对照 → 定性裁定书

**Files（Task 4）:**
- Read: `crates/game-core/src/world/room_draw.rs`（现有 `0x005A` 写字面量写点）、`world/room_load.rs`
- Create: `docs/findings/room-5a-transition-verdict.md`（裁定书，A/B 级分开写）

**Interfaces（Task 4）:**
- Consumes: Task 3 的写序列（值 + 帧 + 可选 pc）、findings §6.4 过渡表、asm 侧 `$5A` 写点（`reference/src/` 或 `refs/m1disasm` 的 `bank:addr`）。
- Produces: 一份裁定书，明确三件事：①`09` 在 ROM 侧由**哪条指令在哪个帧相位**写入；②Rust 侧对应写点是否存在、差在哪个相位；③这是「缺写」「写入时机错」还是「采样相位差」——三者处置不同。

- [ ] **Step 1: 枚举 Rust 生产代码里所有 `$005A` 写点（含间接写）**

```bash
grep -rn '0x005A\|ROOM_NUMBER' crates/game-core/src --include=*.rs | grep -v 'assert' | grep -v '#\[test\]'
```
Expected: 每个写点给出 `文件:行号` + 触发条件（门过渡/row 选择/房间加载）。**只登记实际存在的**，不要按名字推断语义。

- [ ] **Step 2: 在 asm 侧找同一写点的 `bank:addr` 锚**

```bash
grep -rn '\$5A\|\$005A' reference/src/ refs/m1disasm/*.asm 2>/dev/null | head -30
```
Expected: 命中处贴 `bank:addr` + 原指令行。找不到 ⇒ 换 `refs/` 其它轨道或在 `docs/findings/` 记「asm 侧未定位」，**不得**用 Rust 侧反推 asm。

- [ ] **Step 3: 写裁定书（三分类，逐类给证据）**

`docs/findings/room-5a-transition-verdict.md` 必须含：
- 表 1：ROM 写序列（帧 / 值 / pc / 指令）——来自 Task 3；
- 表 2：Rust 写序列（帧 / 值 / 调用点 `文件:行号`）——可临时在 `src/bin/rust_screenshot.rs` 加**启动参数**级 trace，或在既有 `--state-out` 上叠加一次 `capture` 采样；
- 裁定：三分类中的一类 + 置信度 + 反证条件（什么观测会推翻它）。

- [ ] **Step 4: 自查裁定不越界**

逐条检查裁定书：每个结论后面是否有 `bank:addr` / `文件:行号` / trace 行号？有没有把 C 级猜测写成结论？有没有声称未执行的实验？任一不满足 ⇒ 改回假设区。

- [ ] **Step 5: Commit**

```bash
git add docs/findings/room-5a-transition-verdict.md
git commit -m "docs(findings): \$005A 门过渡分歧裁定——ROM 写序列 vs Rust 写点，三分类型定谳"
```

---

### Task 5: 跨模拟器互证（quickerNES ↔ Mesen2，同输入 f=0..1000）

**Files（Task 5）:**
- Create: `analysis/traces/mesen-6725M-1000.jsonl`（不入库）
- Create: `tools/movie_ref/compare_cores.py`（若 `compare_bizhawk_rust.py` 无法直接复用；否则复用）
- Read: `tools/scenario_run.py`（`inputlog` 指令，:279-283）、`oracle/scenarios/` 现有用例

**Interfaces（Task 5）:**
- Consumes: Task 2 修好的 `movies/eien86-6725M.inputlog`、Task 1 的全片轨迹（截前 1001 行）。
- Produces: 一份「同输入下两核心逐帧是否同值」的判定，直接决定既有 Mesen2 基线（`data/status/baselines/`、l4/l3 结论）能否迁移到 quickerNES 语境。

- [ ] **Step 1: 写临时 Mesen2 用例（**放 `/tmp`，不进 `cases/`**）**

内容：`frames 1000` + `inputlog movies/eien86-6725M.inputlog` + 每帧采 `$0106/$0107/$0300/$005A/$0049` 的 `capture` 配置（字段名与 BizHawk 轨迹对齐）。

- [ ] **Step 2: 跑 Mesen2 腿**

```bash
python3 tools/scenario_run.py run --scenario /tmp/core-cmp-1000.sched --out /tmp/mesen-cmp
```
Expected: 产出逐帧 jsonl（或 `*.frames.jsonl`），帧数 ≥1000。
（`scenario_run.py run` 的确切参数以 `python3 tools/scenario_run.py run --help` 为准；DSL 语义见 `oracle/scenarios/README`。）

- [ ] **Step 3: 逐帧比两侧（同帧号直接比，协议同 findings §6.1）**

```bash
python3 tools/movie_ref/compare_cores.py --a analysis/traces/bizhawk-6725M.jsonl \
  --b /tmp/mesen-cmp/core-cmp-1000.frames.jsonl --fields health_lo,health_hi,status,room,scrolldir --start 20 --end 1000
```
Expected: 要么「0 失配」⇒ 两核心在这五字段同源，Mesen2 结论可迁移；要么给出首分歧帧与字段。

- [ ] **Step 4: 若出现分歧，判定「核心差异 vs 观测协议差异」**

对首分歧帧：查该帧两侧 `$0014` 锁存是否同值（输入相位）、查是否落在 `$5A` 过渡段、查是否属上电/复位窗口。任一为真 ⇒ 归因协议/窗口；否则归因核心。结论写进 findings，**不得**用"应该是一样的"糊过去。

- [ ] **Step 5: 更新 findings + KB（若产生可迁移教训）**

- findings 新增 §10「跨模拟器互证」；
- 若发现「核心差异会伪造 L3/L4 红」这类可迁移判断，写 `docs/knowledge/`（type 选 `gotcha` 或 `reference`），并按 README §4 表追加行 + `baseline update --metric knowledge_entries`。

- [ ] **Step 6: Commit**

```bash
git add docs/findings/movie-bizhawk-6725m.md tools/movie_ref/compare_cores.py docs/knowledge/
git commit -m "docs(findings): 跨核心互证 quickerNES↔Mesen2（同输入 f20..1000）——结论与迁移边界"
```

---

## 不在本计划（需单独批准/单独立计划）

1. **修 room 过渡时序**（让 Rust 复现 `$5A=09` 与 ±1~2 帧相位）：属**改游戏逻辑**，必须等 Task 4 裁定书出来再立项；届时按 TDD（先写满 967 帧对拍的失败断言）+ `gates/cli.py trace` 回归，且不得为"变绿"调阈值。
2. **给 `rust_screenshot` 加「全帧状态行」**（消灭 14 帧无采样缝）：属观测协议改造，会动门禁口径，收益是分母完整而非行为正确，优先级低于 Task 1-4。
3. **1000 帧之后的抽样对拍矩阵**：等 Task 1 的全片轨迹到手再规划（每段几百帧、按 TAS 分段跑，避免长跑掩盖首分歧）。
4. **`gotcha-concurrent-*` 的并发护栏自动化**：本轮只用纪律（`pgrep` 检查 + 唯一产物路径），未做机械锁。

## Self-Review 记录

- **Spec 覆盖**：§5 产物 → Task 1；§7 缺陷 → Task 2；§6.4 过渡表 → Task 3/4；§2 核心选择 → Task 5 的迁移边界。§9「未做」里三项已分别落到 Task 1（全片）、Task 3（`$5A` 机制）、Task 5（互证）。
- **步扫描**：每个 step 有唯一动作 + 可核对的期望输出；没有 "handle edge cases" 类空步。Task 3 Step 1 是**探针**（因为 `event.onmemorywrite` 的确切签名未在本机验证过，只有 DLL 字符串 `OnMemoryWrite`/`onmemorywrite` 作线索），这是有意的"先证 API 再写脚本"。
- **类型一致性**：字段名全程 `health_lo/health_hi/status/room/scrolldir/joy1_latch`（与 `bizhawk_ref.lua` 输出、`compare_bizhawk_rust.py` 的 `FIELDS`、`frame_face.rs` 契约一致）；帧轴协议全篇唯一。
- **Review Focus**：5 条各自被钉到 Task 1 Step 3/4（提前结束、半行）、Task 2 Step 2/3（位序第二层）、Task 3 Step 4（写回调漏路径）、Task 1 Step 1（并发）。
- **比例**：本计划约 250 行，配套 spec（findings 315 行）体量相当；无代码正文转写，只给签名/命令/断言。
