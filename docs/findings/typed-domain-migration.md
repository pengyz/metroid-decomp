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
| 3（待做） | `hi/y/x` 位置三元组 | — | — |
| 4（待做） | 动画四元组 `frame/delay/reset/index` | — | — |
| 5（待做） | `radiusY/radiusX`、`isHit`、`onScreen`、`subPixel*` | — | — |
