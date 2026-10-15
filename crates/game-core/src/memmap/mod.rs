//! game-core::memmap —— 映射层：typed state ↔ RAM/WRAM 字节域（two-layer-architecture.md §1）。
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

pub mod cd9c;
pub mod enemy_attrs;
pub mod enemy_slots;
pub mod enemy_todo_0404;
pub mod enemy_todo_040e;
pub mod flow_scratch;
pub mod rom_tables;
pub mod world_map;
