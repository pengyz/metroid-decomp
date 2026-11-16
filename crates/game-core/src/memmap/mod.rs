//! game-core::memmap —— **存储与导出层**：NES 字节容器 + 字节导出面。
//!
//! 分层（2026-11-17 裁定，见 docs/findings/typed-domain-migration.md）：
//! - `storage`：`NesStorage{ram,wram}` 字节容器，`Manager` 私有字段，只此一层摸字节布局；
//! - `observe`：状态→NES 字节的**单向投影**（`snapshot_nes()`），只服务对拍与 `--state-out`；
//! - `world_map`：地图格常量。
//!
//! **语义面不在这里**：实体语义（`samus::state` / `enemy::state` / `title::state`）住在
//! 各自的实体目录，因为它们是聚合根/句柄，不是内存映射的一部分。
//!
//! **地址常量只活在本层**（含 repr/codec）；注释回指 bank:addr 证据。产品层（enemy/ 等）
//! 语义命名零地址字面量；验证适配层（verify_shim）经本层 codec 做状态⇄字节换算，
//! 写集由状态变更推导（不手写地址）。
//!
//! 双口径（lead 裁定）：槽语义视图（16B×16）+ 字节偏移契约面（L2 已证任意 X）并存，
//! encode/decode 往返互检进单测。
//!
//! 迁移注：存量 l2ports 切片转写（sub_F2xx 字面形态）按 §5 留作 verify_shim 参考件，
//! 逐域被语义实现替换后退役——对拍连续性优先，暂不删。
pub(crate) mod storage;
pub mod observe;
pub mod world_map;
