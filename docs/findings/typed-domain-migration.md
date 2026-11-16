# 域 typed 化：把「字节格」换成 Rust 状态，同时不丢 NES 字节判据

状态：进行中（第一批 `$0300`，第二批 `$0308/$0309/$0314/$0315`）

## 0. 为什么要做

`Manager` 目前把整个状态放在两块字节数组里（`NesStorage.ram/wram`）。这在保真期是对的——
它让每个偏移都能被 `ram.hex` 逐字节对拍。但它有三个长期代价：

1. **语义不可表达**：`speedY` 存 `0xFE`，看代码的人必须知道它是二补数才知道这是 -2；
   `status` 存 9，必须查 `sa_*` 表才知道是电梯。
2. **同址多义**：`$0406` 一个字节同时是 `chunkyExplosionTimer`/`movementInstrIndex`/
   `speedSubPixelY`，靠"谁在读"决定含义——这种歧义在字节面永远消不掉。
3. **替换成本集中爆发**：如果等到 Stage B 才动手，改一处要动全仓。

## 1. 机制（三层，缺一不可）

| 层 | 职责 |
|---|---|
| `NesStorage`（`memmap/storage.rs`） | 只装还没迁移的字节；**私有**，字段不出模块 |
| `views::<domain>::<Domain>State` | typed 真相面（枚举 / `i8` / 结构体） |
| `Manager::snapshot_nes()` | **单向投影**：typed → NES 字节，供对拍与 `--state-out` |

关键约束：**一个字段只有一个真相**。迁入 typed 之后，`NesStorage.ram` 里那一格不再被读写，
`read_ram/write_ram` 对该地址转投 typed。这样既保住全部既有消费点（地址面不变），
又让类型系统开始约束新代码。

## 2. 单源映射表（防"两份地址映射漂移"）

```rust
impl SamusState {
    pub const TYPED_ADDRS: [u16; N] = [..];
    pub fn get_byte(&self, addr: u16) -> Option<u8>;   // None = 未 typed 化
    pub fn set_byte(&mut self, addr: u16, v: u8) -> bool;
}
```

`read_ram`、`write_ram`、`Samus` 视图的 `get/set`、`snapshot_nes()`、MSS 回灌**全部**从这张表派生。
新增字段 = 加字段 + 表里加一项 + `get_byte/set_byte` 各一条 match 臂。

## 3. 验收流程（每次迁移一批字段）

1. `cargo test --workspace`（基线只升不降）；
2. `gates/cli.py check` PASS=19；
3. L4 单案（`case-002-door-blue-interaction` 8 格）+ 全矩阵 `run-all`（15 案/132 格）；
4. 投影测试三件套：typed→字节往返、字节→typed 解码、**表与投影一致**；
5. 迁移完成后跑一次 100% 录像窗口（f2160 附近），确认 `frames_synced` 不退。

## 3.5 迁字段的真实成本：找全「读取面」

第三批踩到一个必须记下来的坑：迁完 `hi/y/x` 后，既有锚测
`capture_seam::capture_n_ram_face_matches_state_row_n` 立刻红——
`capture(450) SamusX=0 vs 128`。

原因不是 typed 结构写错，而是 **ram.hex dump 直接遍历 `mem.ram()`（裸存储）**，
而 typed 字段的真相在 `Manager.samus` 里，裸面自然是 0。

所以地址面（`read_ram`）只是**明面上的**读取者；真正的暗面是**整面导出**：

| 导出面 | 位置 | 处置 |
|---|---|---|
| `ram.hex` / `ppu.hex` dump | `crates/game-core/src/bin/rust_screenshot.rs` | 改走 `snapshot_nes()` |
| `TitleFace.ram/wram` | `crates/game-core/src/title/bridge.rs` | 改走 `snapshot_nes()` |
| `--state-out` / `watch_snapshot` | `frame/memory.rs` | 已走投影 |
| `capture_mss` | `frame/memory.rs` | 已走投影 |

结论：**每迁一批字段，先全仓搜 `\.ram()`/`\.wram()` 的直读点**，再动结构——
否则会得到"测试绿但 dump 缺字段"的假绿，而这类假绿恰好会被下游锚测（如 capture_seam）抓住，
但那是在付出一次红测之后。

## 4. 已知陷阱

- **裸字节写会静默失效**：迁入 typed 后 `ram_mut()[0x0300] = v` 不再改变状态。
  有一条测试专门钉这个事实（`raw_byte_write_does_not_override_typed_truth`）——
  它既是契约也是告警：任何残留的裸写都必须清掉。
- **MSS 恢复面**：`restore_mss` 从字节回灌 typed（外部真相面），`capture_mss` 走投影。
  两边漏一个，快照往返就会丢字段。
- **未定义值**：`SamusStatus::Undefined(u8)` 保真保留，不静默映射成 `Stand`。
- **投影不是第二真相**：写进 `snapshot_nes()` 的注释——它只做"状态 → 字节"的单向变换，
  判据仍以像素（L4）与语义（录像/矩阵）为准。

## 5. 进度

| 批次 | 字段 | commit | 测试 |
|---|---|---|---|
| 1 | `$0300 status`（枚举 + `Undefined`） | `9dbbfe07` | 637 |
| 2 | `$0308/$0309/$0314/$0315`（`i8`） | `4043b7e6` | 640 |
| 3 | `$030C/$030D/$030E` → `Position { hi, y, x }`（+ `y16()`） | `c875f3b9` | 641 |
| 4（待做） | 动画四元组 `frame/delay/reset/index` | — | — |
| 5（待做） | `radiusY/radiusX`、`isHit`、`onScreen`、`subPixel*` | — | — |

## 6. 分层裁定（2026-11-17）：typed 类不属于 memmap

用户追问「最新的 typed 类不应该放 view 路径下了吧，它实际上就是语义类实体」，辩证后成立，
但成立的理由不是「它们已经有行为」（当时没有：74 个方法绝大多数是取值/赋值），而是**它们
拥有不变量**：`Anim{frame,delay,reset,index}` 四字节是一个不变量、`Position{hi,y,x}` 是一个
不变量、`Undefined(u8)` 定义「什么算合法」。

### 6.1 身份

Samus = **单例聚合根（singleton aggregate root）**，且必须分开记两个身份：

| 身份 | 含义 | 约束 |
|---|---|---|
| 模型身份＝聚合根 | 一致性边界：22 字节 + gear + 碰撞判据必须一帧内自洽，改动经由它 | 跨域写入必须走 `systems/`，不许绕 |
| 机制身份＝独占借用句柄 | `Samus<'a> { mem: &'a mut Manager }`，不拥有状态 | 不可 Clone、不可持久化、同刻只一个域 |

敌人的聚合根是**池**（12 槽）不是单只敌人：分配、`RemoveEnemy`、跨槽扫描都是池级，
`Manager::enemy(slot)` 只是池成员句柄。

### 6.2 目录

`memmap` 退回「存储 + 导出」：

```
crates/game-core/src/
├── samus/{mod.rs, state.rs, physics.rs, handler.rs, ...}
├── enemy/{mod.rs, state.rs, motion.rs, ai_table.rs, ...}
├── title/{mod.rs, state.rs, bridge.rs, ...}
└── memmap/{mod.rs, storage.rs, observe/, world_map.rs}
```

迁移方式：`git mv memmap/views/{samus,enemy,title}.rs → {samus,enemy,title}/state.rs`，
删 `memmap/views/`，`memmap/mod.rs` 的模块声明去掉 `views`，三条细则的权威位置移到
`samus/state.rs` 头注释（其余域指过来）。

### 6.3 后续两步（尚未落地）

1. `samus/physics.rs` 的纯自域函数搬成 `impl Samus`（球反弹、岩浆径、`CheckStopHorzMvmt`）
   —— 这一步才真正兑现「聚合」，此前只是搬家。
2. `systems/` 收跨域（子弹命中敌人、岩浆写 `HealthChange` + 击退）。

止损：第 1 步两周内不落地 ⇒ 承认只是搬家，不再追加实体化；`no-raw-addr` 门继续按住增量
（本次迁移后账本 810→807，豁免面从 `memmap/views/*` 改为三处 `*/state.rs`）。
