# typed struct 分批计划：NES memory 变成投影

> 目标（Human 定的方向）：**业务层以 Rust struct 为准，NES memory 是投影**，逐步做。
> 本文只出「可验收的分批计划」，不动代码。所有数字都带命令，可在本机复算。
>
> 采样：**HEAD `b1e33a6d`，2026-11-25T12:49:56Z**，工作树 `git status --porcelain --untracked-files=no` 为空。

---

## 0. 一页结论

1. **现状不是「没结构」，而是「结构是派生的」**：`crates/game-core/src/samus/state.rs:182` 已有 `pub struct SamusState`（含 `Position`/`Anim`/`Radius`/`SubPixel` 四个子结构）。> 【2026-11-26 更正】**这句话描述的 2026-11-25 现状已被后续实现推翻**：`samus/state.rs:292-340` 的 `get_byte/set_byte` 已把 `$0300-$0316`（`TYPED_ADDRS` 23 项）与 `$0106/$0107` 分派到 typed 字段，`memmap/raw.rs` 的 `read_ram/write_ram` 在镜像归一时先走 typed 分派，单测 `raw_byte_write_does_not_override_typed_truth`（`state.rs:816-822`）断言「直接写 RAM 不改 typed 真相」。⇒ **对 `read_ram/write_ram` 这条唯一读写口径而言，typed 已是权威、字节数组不再持有这些格**（`snapshot_nes` 反向往 RAM 校验面投影）。原判据与批次划分保留不动，但「RAM 权威」只对**未 typed 的**字节面成立。**[2026-11-26 更新，依据 `sed -n '285,345p' crates/game-core/src/samus/state.rs`、`sed -n '20,80p' crates/game-core/src/memmap/raw.rs`]**
   > 口径提示：卡面提到的 `room_sweep` 在 `tools/` 下不存在（实测 `ls tools/ | grep sweep` 只有 `decode_sweep.py`、`decode_sweep_dyn.py`），本计划一律用后者。
2. **投影 API 已经存在**：`crates/game-core/src/memmap/nesmap.rs` 的 `NesMapped::project(&self, out: &mut NesImage)` 就是「struct → 字节面」的写出口，15 个 typed 面已经在用（`$0000` 零页、`$0400` 对象槽、`$0500` TileBlast、`$0600` 声引擎、`$0700` 房间精灵表、`$0780` VRAM 缓冲、`$068E/$0787/$07F0` 暂存、`$0106` 血量、`$0300` Samus、`$6AFB` 敌、`$6BB4` 敌爆散、`$0400` 与 `$6AFB` 额外面、`$0100-$017F` OAM 影子、`$6B34` 对象全局、`$037A` 滚动块、`$006E` 生命变更） **[2026-11-26 更新，依据 `python3 tools/nes_map_check.py --list` / `grep -rln 'NesMapped for'`：现 Owner 1401 / Unmodeled 385（24 段）/ Undeclared 486（11 段）/ 15 个 typed 面]**。缺的不是机制，是**把业务层的读写点接到它上面**。
3. **最小可行 slice 已选定**：`SamusVitals`（`$006E-$006F` 生命变更 + `$0106-$0107` 当前血量 + `$0301/$0302` 半径）—— 字节数小（6 B）、写点集中、`$0106-$0107` 写作时是 25 段 Unmodeled 里唯一带「health() 仍走字节面」注释的缺口（现 24 段），做它能**同时**验证「权威翻转」和「降 Unmodeled」两件事。
4. **四批按「读多写少 → 热路径」排序**，每批的入场券都是「行为不变」：`cargo test -p game-core` 全绿、L4 171/172 不回退、`run-all` 不新增 diff、`check` PASS24。任一批做不到就回滚该批，不带着红灯进下一批。
5. **本计划不承诺一轮吃完**：剩余 486 B Undeclared / 385 B Unmodeled 的分布见 §1.2 **[2026-11-26 更新，依据 `python3 tools/nes_map_check.py --list` / `grep -rln 'NesMapped for'`：现 Owner 1401 / Unmodeled 385（24 段）/ Undeclared 486（11 段）/ 15 个 typed 面]**，人工与门禁成本都在计划里列了，按批派卡。

---

## 1. 现状实读

### 1.1 已声明的 typed 面（`crates/game-core/src/memmap/`）

```bash
ls crates/game-core/src/memmap/
# mod.rs  nesmap.rs  object_slot_face.rs  observe/  raw.rs  room_sprite_table_face.rs
# sound_ram_face.rs  spare_face.rs  storage.rs  tile_blast_face.rs  verify_shim.rs
# vram_buffer_face.rs  world_map.rs
cargo run -q -p game-core --bin nesmap-dump | python3 -c "…"   # 按实体汇总（见下）
```

| 实体 | 字节 | span 数 | 状态分布 |
|---|---|---|---|
| `Room` | 2053 | 4 | unmodeled 2053 |
| `OamShadow` | 256 | 1 | **owner 256** |
| `Object` | 256 | 2 | view 23 / unmodeled 233 |
| `ObjectSlots` | 256 | 18 | **owner 256** |
| `TileBlasts` | 208 | 13 | **owner 208** |
| `Enemy` | 192 | 24 | owner 84 / unmodeled 108 |
| `SpareScratch` | 155 | 3 | **owner 155** |
| `SoundRam` | 140 | 2 | **owner 140** |
| `RoomSpriteTables` | 128 | 18 | **owner 128** |
| `VramBuffers` | 87 | 4 | **owner 87** |
| `Explosion` | 32 | 1 | unmodeled 32 |
| `SamusState` | 25 | 24 | owner 23 / unmodeled 2（`$0106-$0107` 血量） |
| `Camera` | 7 | 7 | unmodeled 7 |
| `ObjectGlobals` | 5 | 5 | **owner 5** |
| `HealthChange` | 2 | 1 | **owner 2** |
| `SoundInitFlags` | 2 | 1 | **owner 2** |
| `ScrollBlocks` | 2 | 1 | **owner 2** |

**读法**：`owner` = 已 typed 的权威面；`unmodeled` = 声明了但还没 typed（这批字节「属于谁」知道，「长什么样」不知道）；`view` = 同一批字节的第二解释（不单独计和）。

### 1.2 四态与两盏棘轮（实读输出）

```bash
python3 gates/cli.py check 2>&1 | grep -E "nes-map|raw-surface|no-raw-addr|arch-coupling"
```

```
[PASS] nes-map：比对面 2272 B / Owner 1401 B / View 23 B（only 0）/ Unmodeled 385 B / Undeclared 486 B（11 段）；四态恒等式成立 **[2026-11-26 更新，依据 `python3 tools/nes_map_check.py --list` / `grep -rln 'NesMapped for'`：现 Owner 1401 / Unmodeled 385（24 段）/ Undeclared 486（11 段）/ 15 个 typed 面]**
[PASS] raw-surface：裸字节面 1087/1160 处 / 35 文件（新文件不许接入、总量只减不增）；可下调账本 73 处
[PASS] no-raw-addr：裸地址 513/524 处 / 23 文件；具名常量 151/151 处 / 6 文件；可下调账本 11 处
[PASS] arch-coupling：基线 1312 指纹 / 1893 点（HEAD），当前扫描 1306 指纹 / 1887 点，无新增；账本比当前树多 6 条
```

- 恒等式：**`Owner + Unmodeled + Undeclared == 2272`**（`1401 + 385 + 486 = 2272`） **[2026-11-26 更新，依据 `python3 tools/nes_map_check.py --list` / `grep -rln 'NesMapped for'`：现 Owner 1401 / Unmodeled 385（24 段）/ Undeclared 486（11 段）/ 15 个 typed 面]**。View 的 23 B 与 Owner 重叠，**不参与求和**（`View(only)` = 0）。
- `D` 面（数据/声明）还剩 **486 B Undeclared 落在 11 段**、**385 B Unmodeled 落在 24 段** **[2026-11-26 更新，依据 `python3 tools/nes_map_check.py --list` / `grep -rln 'NesMapped for'`：现 Owner 1401 / Unmodeled 385（24 段）/ Undeclared 486（11 段）/ 15 个 typed 面]**，逐段清单：
  ```bash
  python3 tools/nes_map_check.py --list | grep -E "^\[账本\]|^\[未登记\]|^\[Unmodeled\]"
  ```

### 1.3 业务层仍在直写 memory 的规模（按目录计）

```bash
for d in samus enemy world object render title ui sound pickup input ending frame; do
  echo -n "$d "; grep -rn "read_ram\|write_ram\|read_wram\|write_wram" crates/game-core/src/$d --include=*.rs | wc -l
done
```

```
frame 1196 · samus 1051 · world 844 · enemy 697 · title 175 · render 118 · sound 94 · pickup 78 · ui 30 · object 0 · input 0 · ending 0
```

**这条数据决定批次顺序**：`frame/`（引擎骨架）与 `samus/` 是最大两块，但它们也是行为最敏感的；`object/` 已经 0（`$0400-$04FF` 面已 typed）。所以正确顺序是「先做读写点最少的 vitals，再动 samus 热路径，最后才碰 frame 骨架」。

---

## 2. 目标态与边界

### 2.1 三层与「谁是权威」

```
业务层 struct（权威）  ──project()──▶  NES image（投影：ram[0x800] / oam / ppu 影子）
        ▲                                        │
        └────────── 每帧一次 flush ◀─────────────┘   （只对「仍读投影」的字段）
```

- **权威在 struct**：字段写入走 `&mut self`，字节面**不再被业务代码直接读写**。
- **投影在 image**：`NesMapped::project(&self, out: &mut NesImage)` 是唯一「struct → 字节」的写出口（已存在，见 `crates/game-core/src/memmap/nesmap.rs`）。
- **唯一写回入口**：每帧末尾由 `crates/game-core/src/frame/engine.rs` 的 tick 收口调用一次
  ```rust
  memmap::project_all(&mut image, &[&samus.state, &enemy.slots, &world.room /* … */]);
  ```
  （现无 `project_all`，本计划建议新增**一个**函数，禁止各处零散 `project()`——否则「唯一入口」立刻失效。）
- **仍每帧从投影读的字段**（明确豁免，不得 struct 化）：
  1. PPU 寄存器与 `PPUCTRL/PPUMASK` 影子（`$2000-$2001` 语义面）—— 硬件状态，不是游戏状态；
  2. OAM DMA 前后的 `$0200-$02FF`（`OamShadow` 已 owner，但它是**引擎写**的镜像，业务层只读）；
  3. APU 相关（`SoundRam` 已 owner，引擎侧单写）；
  4. 房间/名表缓冲（`Room` 2053 B unmodeled）—— 属渲染管线，本计划不碰。
- **判定规则（互斥，可直接代入）**：
  | 情况 | 判 |
  |---|---|
  | 字段被 ≥2 个业务模块写 | 必须 struct 化（否则不变式无处安放） |
  | 字段只被一个模块写、且每帧必写 | 可以先留投影，但要在 §2.2 清单里登记 |
  | 字段是硬件/引擎私有镜像 | 永远不 struct 化，只读 |

### 2.2 最小可行 slice：`SamusVitals`

选它的三个理由（都可复算）：
1. `$0106-$0107` 是 `SamusState` 面里唯一 **unmodeled** 的 2 B，且 `nesmap-dump` 的 note 直接写着「血量两字节未 typed：`health()/set_health()` 仍走字节面」——做它就是把这个注记清掉；
2. `$006E-$006F HealthChange`（2 B，已 owner）与 `$0301/$0302 Radius`（2 B，已 owner）的读写点极少 —— 实测 `samus/` 下涉及这 4 个地址的**生产代码读点只有 3 处**（`physics.rs:50`、`:981-982`）＋`$0301` 2 处（`:50`/`:1442`），`$0302` 无生产读点，行为风险低；
3. 它是**读多写少**：血量一局改几十次，读的次数高一个量级，适合当「权威翻转」的第一个案例。

| 项 | 值 |
|---|---|
| 覆盖字节 | `$006E-$006F`(2) + `$0106-$0107`(2) + `$0301-$0302`(2) = **6 B** |
| 新 struct | `crates/game-core/src/samus/vitals.rs`：`pub struct SamusVitals { health: u16, health_change: u8, radius: Radius }`（`health` 用 `u16` 承接 `$0106 低 / $0107 高`） |
| 权威翻转点 | **`crates/game-core/src/samus/state.rs:678` 的 `health()` 与 `:682` 的 `set_health()`**（这两个函数就是「血量读写」的现有入口，`frame/memory.rs` 里 `grep health` 只命中 `HealthChange::ADDRS` 的转发行 :317/:370，**不是**权威点）；外加 `crates/game-core/src/samus/physics.rs:50`、`:981-982`（`read_ram(0x0107)`/`read_ram(0x0106)`）、`:1442` 三处直读改为读 struct。`0x0302` 在业务层**没有读点**（`samus/` 下只在 `state.rs:857` 的测试断言里出现），所以 Radius 的翻转只涉及 `$0301` 的 2 处读（`:50`/`:1442`）与 1 处写（`:2199` 测试）|
| 写回 | `impl NesMapped for SamusVitals`，`project()` 只写这 6 B；由 §2.1 的 `project_all` 每帧调一次 |
| 预期账面 | `SamusState` unmodeled 2 → 0；**Unmodeled 385 → 383**，Owner 1401 → 1403（`$0106-$0107` 从 unmodeled 变 owner）；`Undeclared 486` 不变 **[2026-11-26 更新：本行原写 387→385 / 1348→1350 / 537，是写作时读数；该批已实现，实测已到 1401/385/486]** |
| 入场券 | `cargo test -p game-core` 全绿、L4 `171/172` 不回退、`run-all` diff 不新增、`check` PASS24 |

> 注意：这一批**只降 Unmodeled，不降 Undeclared**——因为那 2 B 已在账本里登记为 unmodeled。这是刻意的：先证明「翻转机制」可用，再去做真正降 Undeclared 的批次。

---

## 3. 四批迁移（读多写少 → 热路径）

排序原则：**每批的失败半径要小于上一批**。B1 只碰 6 B，B4 才碰帧骨架。

### B1 · `SamusVitals`（§2.2 的 slice）

- 文件：新增 `crates/game-core/src/samus/vitals.rs`；改 **`crates/game-core/src/samus/state.rs`**（`health()`/`set_health()` :678/:682 的权威翻转）、`crates/game-core/src/samus/physics.rs`（:50/:981-982/:1442 三处直读）、`crates/game-core/src/memmap/{mod,nesmap}.rs`。`frame/memory.rs` 只在 `HealthChange::ADDRS` 的转发面上受影响（:317/:370），按实现需要决定是否动。**`samus/handler.rs` 不在本批**：它 :439/:495/:1923 读的是 `0x0300 + s` 状态槽，属 B2 的 `$0300-$0316`，不在 `$0106/$0107/$0301/$0302` 这 6 B 里。
- 判据：`cargo test -p game-core` 全绿（**不设自造门槛**，跟门禁读数比：`check` 的 `rust-tests` 行当前为「测试数 1175（基线 1175）」）；`python3 tools/l4_diff_run.py run cases/case-password-justin.txt` 仍 14/14；`run-all` 仍 `171/172`；`check` PASS=24。
- 回滚：`git revert` 单个 commit 即回到「RAM 权威」形态（B1 的血量 2 B 与 B2 的位姿 23 B 各自一个 commit，可单独回滚），无数据迁移。 **[2026-11-26 更新]**
- 耦合：`data/status/nes_map.json` 的 `entity_spans` 计数 17 → 18（若新建实体）或 `SamusState` spans 24 → 22（若并入）；`arch_coupling_allowlist.json` 若在 `samus/` 内新增直接内存调用则会涨点——本批是**减少**调用，预期不涨。

### B2 · Samus 位置/速度热路径（`$0300-$0316` 余下 23 B）

- 文件：`crates/game-core/src/samus/state.rs`（已有 `SamusState`，把它从「派生视图」改为「权威」）、`samus/{physics,control,integrators}.rs`、`frame/memory.rs`。
- 判据：同 B1，另加 `case-elev-norfair-transition` 10/10（该案对 Samus 位姿最敏感）。
- 回滚：该批拆成 3 个 commit（位置 / 速度 / 子像素），逐个可 `revert`。
- 耦合：`raw-surface` 基线 1087 应下降（调用点变 struct 字段访问）；`arch_coupling` 的 `sym` 类点数同向下降——**两盏棘轮都是只降不升，本批方向正确**。

### B3 · 敌人槽（`Enemy` unmodeled 108 B + `Explosion` 32 B + `$0317-$03FF` 233 B）

- 文件：`crates/game-core/src/enemy/state.rs`（已有 `ENS_BASE` 与 24 span）、新增 `enemy/extra_face.rs`（`$6AFB-$6BD3` 13 段共 140 B）、`world/room_draw.rs` 的 `load_enemy`。
- 判据：`cargo test -p game-core`；L4 全 172 格不回退；`case-crawlprobe` 与 `case-mechanism-input-conflict` 逐格不新增。
- 回滚：按面回滚（extra 面独立于主槽面）。
- 耦合：本批**会降 Unmodeled 140+108+32 = 280 B → Owner**；`nes_map.json` 需 `--write` 重生成并写 commit 说明（转换等式：`delta_owner + delta_view == -(delta_unmodeled + delta_undeclared)`）。

### B4 · 帧骨架与零页（`frame/` 1196 处 + `$0000-$0032` 51 B + `$0108-$01FF` 248 B）

- 前置：B1–B3 全绿，且 `project_all` 已稳定运行 ≥1 周无回归。
- 文件：`crates/game-core/src/frame/{tick,nmi,engine,memory}.rs`、零页面新文件。
- 判据：L4 全格 + `run-all` 不新增 + **TAS 首分歧不提前**。当前基线取 `analysis/tas-main-f22895/coverage.json` 的 `first_divergence = {frame:22895, field:"$0400", emu:152, rust:151, diff_bytes:11}`；#71（A-1）验收后 f22895 的 `$0400/$0401/$040A` 已归零、剩 8 字节是 `$0405` 相位差（A-2 承接）。
  > 旧文写「首分歧 `f2084`（`$0415`）」已过期——那是 #55 修复之前的锚点，照它执行会拿一个早已修过的帧当基线。
  > 复算：`python3 -c "import json;d=json.load(open('analysis/tas-main-f22895/coverage.json'));print(d['first_divergence'])"`。
  另加 `decode_sweep.py` 对照（本仓无 `room_sweep`）。
- 回滚：本批最容易把「帧相位」做坏，**必须**先有一版 `project_all` 的隐式一致性测试（同一帧 project 前后 `snapshot_nes()` 逐字节相等）。
- 耦合：`$0108-$01FF` 里有 6502 栈，**只能 Ephemeral**（不计降幅，见 §4 禁止项），真正可 typed 的只有游戏变量，需先做写点普查。

---

## 4. 风险与禁止项

1. **不许一次性重写 `crates/game-core`**：本计划是 4 批 × 若干 commit，每次翻转一小块权威面。
2. **不许绕开差分判据**：每批入场券是 `cargo test` + L4 + `run-all` + `check`；「代码看起来对」不算完成（AGENTS §1）。
3. **不许在业务层新增裸地址常量**：`no-raw-addr` 与 `raw-surface` 两盏棘轮只降不升；新面必须复用既有 `pub(crate) const`（`enemy/state.rs:271 ENS_BASE`、`enemy/explosion.rs:15 BASE` 是先例）。
4. **不许用 Ephemeral 降 Undeclared**：`tools/nes_map_check.py:903-915` 的转换等式决定了「下降只能来自转成 Owner/View」；栈区（`$0100-$01FF`）按 Ephemeral 登记，不计降幅。
5. **不许投影与 struct 双写**：一旦某字段权威翻转，业务层**不得**再 `write_ram` 同一字节（双写会产生「谁最后写谁赢」的隐性 bug）。检查办法：`grep -rn "write_ram(0x0106\|write_ram(0x0301" crates/game-core/src/` 应为 0。
6. **不许把「struct 存在」当完成**：`samus/state.rs:182` 的 `SamusState` 早就存在，但它今天是派生的——判据是「谁写、谁读」，不是有没有 struct。

---

## 5. 与账本/门禁的耦合点（每批都要看）

| 耦合物 | 位置 | 本计划的影响 |
|---|---|---|
| `data/status/nes_map.json` | 四态账本 | 每批结束跑 `python3 tools/nes_map_check.py --write`（只许下调），并在 commit message 写 before/after 两行 |
| `data/status/arch_coupling_allowlist.json` | `arch-coupling` 基线 1312 指纹 / 1893 点 | 本计划方向是「减少直写」，预期逐批下降；**不得**为过门禁往账本追加条目（#28 堵的就是这条路） |
| `raw-surface` 基线 1087/1160 | `tools/raw_surface.py` | 同上，只降不升 |
| `no-raw-addr` 513/524 | `tools/no_raw_addr.py` | 新面若引用裸地址会红；用既有 `pub(crate) const` |
| `L4` 矩阵 171/172 | `shots_rust/matrix.json`（带 `head`/`at`，见 `docs/design/evidence-binding.md`） | 每批跑 `run-all`，diff 不新增；矩阵必须带 head/at 才能当证据 |
| 房间对照 | `tools/decode_sweep.py` / `decode_sweep_dyn.py`（`tools/` 下**没有** `room_sweep`，卡面写的这个名字在本仓不存在，别照抄） | B3/B4 用它做房间解码对照，不新增工具 |

---

## 6. 待决（需要 PMO 拍一句）

1. **`project_all` 的落点**：放在 `frame/engine.rs` 的 tick 收口（每帧一次），还是每个 face 自己管？本计划推荐前者（唯一入口），但这会动 `frame/`，属 B4 范围——需要确认「B1 阶段可以先在 `frame/memory.rs` 内联 project，等 B4 再统一」。
2. **`SamusVitals` 是并入 `SamusState` 还是独立文件**：并入则 `entity_spans` 计数不变、但 `SamusState` 面变成「混合权威」；独立则账面清晰、但多一次 project 调用。本计划推荐**独立文件**（B1 少动既有结构）。
3. **B3 的 `$0317-$03FF`（233 B）是否需要先补 asm 取证**：该段是每区状态数组尾，`nesmap-dump` 没有 note。若 PMO 要求「无 A 级证据不得 typed」，则 B3 拆成「先取证卡 + 后实现卡」。

## 状态同步（2026-11-26，HEAD `4e7ce4de`）

本文写作时（#74，2026-11-25）是计划；现在按实际进度回填**状态**，不改上面任何判据与批次划分。

| 批次 | 状态 | 事实（可复跑） |
|---|---|---|
| B1 `SamusVitals` 6 B | **已完成** | 实际只收了可收的 2 B：`$0106/$0107`（asm `Health`，`refs/m1disasm/SRC/constants_ram.asm:806`）。已落 `crates/game-core/src/memmap/samus_vitals_face.rs` + `samus/state.rs` 权威翻转（`bdba1b0f`，#77）。卡面另两个字段经验伪：`$006E/$006F` 是 `HealthChange`（早已 typed）、`$0301/$0302` 是 `radiusY/radiusX`（属 B2 的 23 B） |
| B1 其余声明面（原计划未细分） | **已完成 6 个面** | `ZeroPage` `$0000-$0032` 51 B（`d2070cb7`，#81）、`ObjectSlots` 256 B + `SoundRam` 140 B（#29）、`SpareScratch` 155 B（#52）、`TileBlasts` 208 B、`RoomSpriteTables` 128 B + `VramBuffers` 87 B。运行时核：`python3 tools/nes_map_check.py --list` → `Owner 1,401 / Unmodeled 385 / Undeclared 486` |
| B2 Samus 位姿 23 B | **已完成（读写口径），剩「业务层直读点收口」** | `samus/state.rs:292-340` 的 `get_byte/set_byte` 已分派 `$0300-$0316` 全部 23 项；`memmap/raw.rs` 的 `read_ram/write_ram` 先走 typed 分派，单测 `raw_byte_write_does_not_override_typed_truth`（`state.rs:816-822`）钉死「typed 是真相」；**未完成的部分**是业务层仍有 `samus/physics.rs` 等处的直读点（走 `read_ram` 已自动 typed，但少一层类型），以及 `$0301/$0302` 的语义命名在 `Radius` 里。 **[2026-11-26 更新，依据 `grep -n 'TYPED_ADDRS' -A3 crates/game-core/src/samus/state.rs`、`sed -n '20,80p' crates/game-core/src/memmap/raw.rs`]** |
| B3 敌人槽/`$0317-$03FF` | **未开工（仅声明层完成）** | `Enemy` 额外面 84 B 与 `Enemy`/`Explosion` 主面已由 `object_slot_face.rs` 与 `enemy/state.rs` 声明，但 `$0317-$03FF` 233 B 仍是 Unmodeled（AGENTS §3.2 要求先取证再命名，PMO 已裁定） |
| B4 帧骨架与零页 | **部分（零页已完成）** | `$0000-$0032` 51 B 已 typed；`$0108-$01FF` 248 B 是 6502 栈区，只能当 Ephemeral，不参与 Undeclared 降幅 |

**`project_all` 尚未落地**（§6 待决 1 的现状）：现在没有统一入口，实际接线是两处手工链——`crates/game-core/src/frame/memory.rs` 的 `TypedRestoreManifest::ram_addrs()`（回灌）+ `crates/game-core/src/state.rs` 的 `Manager::snapshot_nes`（投影）。每次新增 face 要在这两处各加一段，**这是 B4 之前已知的维护成本**，PMO 在 #77 裁定「B1 可先内联，B4 再统一入口」。

**复跑命令**：`python3 tools/nes_map_check.py --list`（四态）、`cargo test --workspace`（1,193，ratchet 基线 1,189）、`python3 gates/cli.py check`（PASS24/0/3/1）。

