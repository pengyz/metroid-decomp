//! game-core::enemy —— 敌方域产品层（two-layer-architecture.md §1：struct + method，零地址字面量）。
//!
//! 命名纪律（AGENTS §3）：语义命名经 data/naming 台账（A 级地址自证 / B 级语义概要级）；
//! 未语义考证字段保留 `_todo_` 前缀 + 触达证据注——禁伪语义命名，宁缺勿滥。
//! F2CA 族九入口 = 本域方法集素材（decode-sweep r1 §3；切片考证=单元 spec 输入）。

pub mod attrs;
pub mod common;
pub mod slot;
pub mod enemy_collision_flags;
pub mod enemy_state;

pub use attrs::EnemyAttrs;
pub use slot::EnemySlot;
pub use enemy_collision_flags::EnemyCollisionFlags;
pub use enemy_state::EnemyState;
