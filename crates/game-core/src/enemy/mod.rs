//! game-core::enemy —— 敌方域产品层（two-layer-architecture.md §1：struct + method，零地址字面量）。
//!
//! 命名纪律（AGENTS §3）：语义命名经 data/naming 台账（A 级地址自证 / B 级语义概要级）；
//! 未语义考证字段保留 `_todo_` 前缀 + 触达证据注——禁伪语义命名，宁缺勿滥。
//! F2CA 族九入口 = 本域方法集素材（decode-sweep r1 §3；切片考证=单元 spec 输入）。

pub mod state;
pub mod common;
pub mod slot;
pub mod movement;

pub use slot::EnemySlot;
pub use movement::{MoveCheckOutcome, MoveCheckStaging};
pub mod crawler;

/// waver 模块（TDD 流程（测试先行（实现后补）））——refs asm 考证：bank01:97D7（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498））（Waver 例程（实址（A 级原文）））

/// mellow 模块（TDD 流程（测试先行（实现后补）））——refs asm 考证：bank07:EE86（SpawnMellow）+bank07:EE63（SpawnMellows）（Mellow 例程（实址（A 级原文）））
pub mod ai_table;
pub mod motion;
pub mod prep;
pub mod slot_cycle;
pub mod tables;
pub mod framework;
pub(crate) mod explosion;
pub mod anim;
pub(crate) mod anim_idx;
pub mod skree;
pub mod sidehopper;

pub(crate) mod waver;
