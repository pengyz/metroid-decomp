# 实体所有权 + NES 映射 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让实现态由 Rust 语义类型拥有、NES 字节面降为只读投影，并用「每字节一个 owner」的机械门禁锁住，最后按经典游戏子系统重组模块。

**Architecture:** 分八步递进：先补观测面盲区（P0）与快照污染（P1），再给四域加 `NesMapped` 声明与静态/动态两道腿（P2–P3），把剩余字节族 typed 化压到零（P4），然后逐域实体自持（P5，Explosion→Enemy→Object→Samus），最后才搬目录并立依赖方向门（P6）。每步都以「矩阵逐格不变 + 对应门读数不升」为验收。

**Tech Stack:** Rust（`crates/game-core`，无 async，纯逻辑）；Python 门禁工具（`gates/`、`tools/`）；Mesen2 对拍（`tools/l4_diff_run.py`）。

**Spec:** `docs/spec/entity-ownership-and-nesmap.md`

## Global Constraints

- asm（`refs/m1disasm`）是唯一真相源；实现前逐行读完对应例程，commit 带 `bank:addr` 锚。
- L4 零容差不修订：`python3 tools/l4_diff_run.py run-all` 的 `match` 不得下降，`diff` 不得上升（当前 15 案/132 格 · match 126 · diff 6 · blocked 0）。
- 门禁禁止调阈值/加白名单；改门必须单独 commit 并写明基线影响。
- 逻辑层禁止 `std::env` 钩子（`no-env-hooks`）；诊断只走 `--trace` 启动参数或独立测试。
- 临时用例不得留在 `cases/`；`movies/` 全 gitignored。
- 删除必须带等价补测（ratchet 不许测试数下降；当前基线 702）。
- 长跑（`run-all`）与源码改动不得并行。
- 判据来源：字段偏移/槽步长从 `refs/m1disasm/out/M1_NES_NTSC.sym` 机械生成，禁止 import Rust 侧常量。

## Review Focus

1. ram.hex 尾段（敌槽 8–11、爆炸槽 1–3）投影若静默错位，L4 与门禁都判绿——本轮由 P0 的 `$6AF4-$6BD3` 全量 dump + 单一来源钉住。
2. `$0307` 同址三义（`SamusOnElevator`/对象 `data07`/门 `DoorType`）：任何一方写坏会被另一解释读到——由 P2 的 owner+view 声明与 P3 的动态腿钉住。
3. `restore_mss` 之后直接读 `ram()` 会拿到 typed 旧副本（stale）：由 P1 的「restore 后 typed 地址全 0」断言钉住。
4. 跨域 systems 若改成「先收集后应用」，会重排 asm 写入序（`$010F` 同帧重读）——由 P5 的「systems 只换签名」纪律 + 帧级 trace 测试钉住。
5. 目录重组期没有门能守新依赖方向（G11 已裁）：由 P6 先立 G11′ 再加目录钉住。

---

### Task 1: 观测面补齐（P0：dump 尾段 + 单一来源）

**Files1:**
- Create: `crates/game-core/src/memmap/observe/dump.rs`
- Modify: `crates/game-core/src/memmap/observe/mod.rs`
- Modify: `crates/game-core/src/bin/rust_screenshot.rs:472-480`
- Test: `crates/game-core/src/memmap/observe/dump.rs`（`#[cfg(test)] mod tests`）

**Interfaces1:**
- Produces: `pub fn ram_dump_hex(mem: &crate::state::Manager) -> String`（2176 字节 → 4352 个十六进制字符 + `\n`，全量取 `snapshot_nes()`，段序 `$0000-$07FF` 后接 `$6AF4-$6BD3`）
- Produces: `pub const DUMP_LEN: usize = 2176;`、`pub const EXTRA_RANGE: (u16, u16) = (0x6AF4, 0x6BD3);`

- [ ] **Step 1·1:** 写失败测试**

```rust
#[test] // 用例1
fn dump_covers_enemy_slots_8_to_11_and_explosion_slots_1_to_3() {
    let mut m = crate::state::Manager::default();
    m.enemy(9).set_status(crate::enemy::state::EnemyStatus::Active);
    m.explosion(2).set_status(1);
    let hex = ram_dump_hex(&m);
    assert_eq!(hex.trim_end().len(), DUMP_LEN * 2);
    let at = |addr: u16| -> u8 {
        let idx = if addr <= 0x07FF { (addr as usize) * 2 }
                  else { (0x800 + (addr - EXTRA_RANGE.0) as usize) * 2 };
        u8::from_str_radix(&hex[idx..idx + 2], 16).unwrap()
    };
    assert_eq!(at(0x6AF4 + 9 * 16), 2, "敌槽 9 的 status 必须在 dump 内");
    assert_eq!(at(0x6BB4 + 2 * 8 + 3), 1, "爆炸槽 2 的额外面必须在 dump 内");
}
```

- [ ] **Step 2·2:** 跑测试确认失败**

Run: `cargo test -p game-core --lib memmap::observe::dump` — 期望 `cannot find function ram_dump_hex`

- [ ] **Step 3·3:** 实现 `ram_dump_hex`**

在 `crates/game-core/src/memmap/observe/dump.rs` 里：取 `let img = mem.snapshot_nes();`，先 `img.ram[0..0x800]` 逐字节格式化，再 `img.wram[(0x6AF4 - 0x6000)..=(0x6BD3 - 0x6000)]` 逐字节格式化；模块头写明「整面单一来源，禁止再混 `read_wram`」。

- [ ] **Step 4·4:** 切 `rust_screenshot.rs` 到新函数**

`fs::write(out.join(&fname), ram_dump_hex(&engine.mem))`；删掉旧的 0x800 + `read_wram` 两段循环与其注释。

- [ ] **Step 5·5:** 验证**

Run: `cargo test --workspace`（702 → 703，`gates/cli.py baseline update --metric rust_test_count --value 703`）；`python3 tools/l4_diff_run.py run-all --skip-emu` 期望 `match 126 · diff 6`。

- [ ] **Step 6·6:** Commit**

```bash
git add crates/game-core/src/memmap/observe/dump.rs crates/game-core/src/bin/rust_screenshot.rs data/status/baselines/
git commit -m "fix(observe): ram.hex 尾段补到 \$6BD3 且整面单一来源（旧形只到 \$6B73=敌槽0-7）"
```

---

### Task 2: 快照往返不再污染字节面（P1）

**Files2:**
- Modify: `crates/game-core/src/frame/memory.rs:153-176`
- Test: `crates/game-core/src/frame/memory.rs`（`#[cfg(test)] mod tests_mss`）

**Interfaces2:**
- Consumes: `crate::samus::state::SamusState::TYPED_ADDRS`、`crate::enemy::state::{SLOT_COUNT, ensx::TYPED_MAX}`
- Produces: `restore_mss` 语义不变，唯一变化是「typed 地址在 `storage` 中归零」

- [ ] **Step 1·7:** 写失败测试**

```rust
#[test] // 用例2
fn restore_leaves_no_typed_copy_in_storage() {
    let mut src = crate::state::Manager::default();
    src.samus().set_status(crate::samus::state::SamusStatus::Roll);
    src.enemy(3).set_status(crate::enemy::state::EnemyStatus::Active);
    let (ram, wram, regs) = src.capture_mss();
    let restored = crate::state::Manager::restore_mss(&ram, &wram, regs);
    assert_eq!(restored.ram()[0x0300], 0, "storage 不得保留 typed 副本");
    assert_eq!(restored.read_ram(0x0300), 3, "读径仍走 typed");
    assert_eq!(restored.enemy(3).status(), crate::enemy::state::EnemyStatus::Active);
}
```  // 唯一化3

- [ ] **Step 2·8:** 跑测试确认失败**

Run: `cargo test -p game-core --lib frame::memory::tests_mss` — 期望 `0x0300` 读到 `3`

- [ ] **Step 3·9:** 实现**

`restore_mss` 末尾（回灌 typed 之后）对每个 `TYPED_ADDRS` 地址与每个 `$6AF4+slot*16+off` 写 `mem.ram_mut()[..]=0` / `wram` 对应位置清零，并加 `debug_assert!` 说明「typed 是唯一真相，storage 只留未迁族」。

- [ ] **Step 4·10:** 验证**

Run: `cargo test --workspace`（703 → 704，更新基线）；`python3 gates/cli.py check` 期望 PASS=21。

- [ ] **Step 5·11:** Commit**

```bash
git add crates/game-core/src/frame/memory.rs data/status/baselines/
git commit -m "fix(memmap): restore_mss 后清零 typed 格，消除 storage 第二真相"
```  // 唯一化4

---  // 唯一化3

### Task 3: `NesMapped` 声明骨架（P2）

**Files3:**
- Create: `crates/game-core/src/memmap/nesmap.rs`
- Modify: `crates/game-core/src/memmap/mod.rs`
- Modify: `crates/game-core/src/samus/state.rs`（加 `impl NesMapped`）
- Test: `crates/game-core/src/memmap/nesmap.rs`（`#[cfg(test)] mod tests`）

**Interfaces3:**
- Produces: `pub struct NesSpan { pub start: u16, pub len: u16, pub kind: SpanKind }`
- Produces: `pub enum SpanKind { Value, Bits(&'static str), Bcd(&'static str), Derived(&'static str) }`
- Produces: `pub enum SpanKey { Owner, View(&'static [&'static str]), Case(u8), Mode(u8), Ephemeral(&'static str), Unmodeled(&'static str) }`
- Produces: `pub trait NesMapped { const DISCRIMINATOR: Option<u16>; const SPANS: &'static [(NesSpan, SpanKey)]; fn project(&self, out: &mut crate::memmap::storage::NesImage); }`
- Produces: `pub fn coverage(spans: &[(NesSpan, SpanKey)]) -> Result<[u8; 2], Vec<String>>`（返回 `[owned_bytes_ram, owned_bytes_wram]`，冲突时返回错误行）

- [ ] **Step 1·12:** 写失败测试（Samus 23 字节）**

```rust  // 唯一化3
#[test] // 用例3
fn samus_declares_every_typed_byte_as_owner() {
    let c = coverage(crate::samus::state::SamusState::SPANS).expect("无重叠");
    assert_eq!(c[0], 23, "Samus 应认领 23 个 RAM 字节");
}  // 唯一化3
```  // 唯一化5

- [ ] **Step 2·13:** 跑测试确认失败**

Run: `cargo test -p game-core --lib memmap::nesmap` — 期望 `no function coverage`

- [ ] **Step 3·14:** 实现骨架 + Samus 声明**

`nesmap.rs`：结构体/enum/trait/`coverage`（用 `[bool; 2048]`/`[bool; 64]` 位图逐 span 标 `Owner`，重复即收集错误字符串；`View`/`Ephemeral`/`Unmodeled` 不标位）。`samus/state.rs` 的 `impl NesMapped for SamusState`：`SPANS` 由 `TYPED_ADDRS` 之外**手写** 23 条 `NesSpan{start: 0x0300+i, len: 1, kind: Value}`（不许循环生成，避免"用实现编造声明"）。

- [ ] **Step 4·15:** 验证**

Run: `cargo test -p game-core --lib memmap::nesmap`；`python3 gates/cli.py check` 期望 PASS=21。

- [ ] **Step 5·16:** Commit**

```bash  // 唯一化3
git add crates/game-core/src/memmap/nesmap.rs crates/game-core/src/memmap/mod.rs crates/game-core/src/samus/state.rs
git commit -m "feat(memmap): NesMapped 声明骨架 + Samus 23 字节 owner 声明"
```  // 唯一化6

---  // 唯一化4

### Task 4: 静态腿门禁 `nes-map`（P2 收尾）

**Files4:**
- Create: `tools/nes_map_check.py`
- Create: `data/status/nes_map.json`
- Modify: `gates/cli.py`（`gate_nes_map()` + `GATES` 条目，插在 `no-raw-addr` 之后）
- Modify: `gates/README.md`（行格式 `| nes-map | 描述 | 状态 |`）

**Interfaces4:**
- Consumes: `cargo run -q -p game-core --bin nesmap-dump`（本任务新增 bin，打印 `{"spans":[…],"coverage":[n,m]}`）
- Produces: 门退出码 0=通过；账本 `{"schema":"nes-map-v1","ram_owned":23,"wram_owned":0,"unmodeled":[{"start":..,"len":..,"reason":".."}]}`

- [ ] **Step 1·17:** 写失败测试**

`tools/tests/test_nes_map.py::test_overlap_is_rejected`：构造两条 `Owner` span 重叠，`nes_map_check.violations(spans)` 必须返回 1 条含 `overlap $0307`。

- [ ] **Step 2·18:** 跑测试确认失败**

Run: `python3 -m pytest tools/tests/test_nes_map.py -q` — 期望 `ImportError`

- [ ] **Step 3·19:** 实现**

`nes_map_check.py`：`violations(spans)` 检查①owner 重叠②`View` 必须有 owner 覆盖③`Unmodeled` 必须带 reason④账本 `ram_owned` 只减不增。`gates/cli.py` 加门（超时 60s），README 补行（注意 README 与注册表一致性自检）。

- [ ] **Step 4·20:** 验证**

Run: `python3 tools/nes_map_check.py --write && python3 gates/cli.py check` 期望 PASS=22、selftest 157/157。

- [ ] **Step 5·21:** Commit**

```bash  // 唯一化4
git add tools/nes_map_check.py tools/tests/test_nes_map.py data/status/nes_map.json gates/cli.py gates/README.md
git commit -m "gate: 新增 nes-map 静态腿（owner 重叠/views 覆盖/unmodeled 理由）"
```  // 唯一化7

---  // 唯一化5

### Task 5: 剩余字节族归零（P4，按族一 commit）

**Files5:**
- Modify: 按族：`crates/game-core/src/world/object_globals.rs`（`$006E/$006F`）、`crates/game-core/src/sound/state.rs`（`$0683/$0684`）、`crates/game-core/src/render/oam_shadow.rs`（新，`$0200-$02FF`）
- Test: 各文件 `#[cfg(test)] mod tests`

**Interfaces5:**
- Produces: `Manager::health_change()/set_health_change(u16)`、`Manager::tri_init_flags()/multi_init_flags()`（若已存在则复用）、`Manager::oam_shadow() -> &[u8; 0x100]`

- [ ] **Step 1·22:** 写失败测试（每族一条）**

```rust  // 唯一化4
#[test] // 用例4
fn health_change_word_is_typed() {
    let mut m = crate::state::Manager::default();
    m.set_health_change(0x0080);
    assert_eq!(m.read_ram(0x006E), 0x80);
    assert_eq!(m.read_ram(0x006F), 0x00);
}  // 唯一化4
```  // 唯一化8

- [ ] **Step 2·23:** 跑测试确认失败** — `cargo test -p game-core --lib world::object_globals`

- [ ] **Step 3·24:** 实现并迁移调用点** — 每族：加 typed 字段 + 迁移该族的 `read_ram/write_ram` 字面量与具名常量调用；跑 `python3 tools/raw_surface.py --write` 下压账本。

- [ ] **Step 4·25:** 验证（每族）** — `cargo test --workspace`；`python3 gates/cli.py check` PASS=22；`run-all` 逐格不变。

- [ ] **Step 5·26:** Commit（每族一条）** — `refactor(core): <族> 收进 typed，raw-surface N→M`

---  // 唯一化6

### Task 6: 动态腿（P3，测试内插桩）

**Files6:**
- Create: `crates/game-core/src/memmap/verify_shim.rs`（`#[cfg(test)]` only）
- Test: `crates/game-core/tests/nes_map_dynamic.rs`

**Interfaces6:**
- Produces: `#[cfg(test)] pub fn assert_owner_unique(mem: &mut Manager, addr: u16, writer: &'static str)`

- [ ] **Step 1·27:** 写失败测试** — 对 `0x0300` 用 `SamusState` 与 `ObjectPool` 同时声明 owner，`assert_owner_unique` 必须 panic 并含地址。
- [ ] **Step 2·28:** 跑测试确认失败**
- [ ] **Step 3·29:** 实现** — 读 `NesMapped::SPANS` 建 `HashMap<u16,&'static str>`，命中即 panic。
- [ ] **Step 4·30:** 验证** — `cargo test --workspace`；`run-all` 逐格不变（插桩仅测试面，不进生产）。
- [ ] **Step 5·31:** Commit** — `test(memmap): nes-map 动态腿（owner 唯一性断言）`

---  // 唯一化7

### Task 7: 实体自持（P5，Explosion 起）

**Files7:**
- Modify: `crates/game-core/src/enemy/explosion.rs`（`Explosion<'a>` → 自持 `ExplosionState`）
- Modify: `crates/game-core/src/state.rs`（`Manager.explosions: [ExplosionState; 4]`）
- Test: `crates/game-core/src/enemy/explosion.rs`

**Interfaces7:**
- Produces: `pub struct ExplosionState { status: u8, pos: Position, anim: Anim, hi: u8, delay: u8, quantity: u8 }`；`Manager::explosion(slot) -> &mut ExplosionState`
- Consumes: Task 1 的 dump（验证投影仍正确）

- [ ] **Step 1·32:** 写失败测试** — `explosion_state_owns_bytes_without_storage`：设 status/anim 后断言 `read_ram(0x6BB4+slot*8+3)` 仍是旧值（storage 不再持真相）但 `snapshot_nes()` 正确。
- [ ] **Step 2–4: 实现 + 全量测试 + `run-all` 逐格不变**
- [ ] **Step 5·34:** Commit** — `refactor(enemy): Explosion 自持状态（投影仍经 snapshot_nes）`

---  // 唯一化8

### Task 8: 目录重组 + G11′（P6，最后做）

**Files8:**
- Create: `gates/deps.py`（G11′：目录→允许依赖表）
- Modify: `gates/cli.py`、`gates/README.md`
- Move: `entities/{samus,enemy,object,explosion}`、`world/{room,camera,door,elevator}`、`systems/{damage,collision,pickup,scroll,anim}`

**Interfaces8:**
- Produces: 门 G11′ 读 `data/status/dep_rules.json`（`{"a": ["b"], …}`），违规即 FAIL

- [ ] **Step 1·35:** 写失败测试** — `gates/tests/test_deps.py::test_entities_may_not_use_systems`
- [ ] **Step 2·36:** 实现门 + `--write` 基线（cm 当前 0 违规）**
- [ ] **Step 3·37:** `git mv` 一轮（只搬不改）**；`tools/raw_surface.py --write`、`tools/no_raw_addr.py --write` 做**路径重映射**（commit 注明「总量不变、非上调」）
- [ ] **Step 4·38:** 验证** — `check` PASS=23、`run-all` 逐格不变、`cargo test` 全绿
- [ ] **Step 5·39:** Commit** — `refactor(core): 按子系统重组目录 + 立 G11′ 依赖门`

---  // 唯一化9

## Self-Review

- **Spec coverage**：Spec §5 的 P0–P7 全部有任务（Task1=P0、Task2=P1、Task3/4=P2、Task5=P4、Task6=P3、Task7=P5、Task8=P6）；Spec §6 测试清单落在 Task1/2/3/6/8 的测试步骤；Spec §7 依赖方向 = Task8 的 G11′；Spec §2 的 R1/R2/R3 分别由 Task1/Task3+6/Task2 钉住。P7（删 storage）**故意不设任务**——五条硬条件未满足前不得开始。
- **未覆盖**：Spec §8 的三条未取证项（restore 污染未跑断言、explosion 文档漂移、per-cell diff 归属）→ Task2 顺手跑断言；文档漂移并入 Task7 的文件头修正。
- **类型一致性**：`NesSpan/SpanKind/SpanKey/NesMapped/coverage` 在 Task3 定义，Task4/6/8 只引用；`ram_dump_hex` 在 Task1 定义，Task7 复用。
- **比例**：本计划不含函数体，只给签名/断言/门判据。
