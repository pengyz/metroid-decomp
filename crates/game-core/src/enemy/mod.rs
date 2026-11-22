//! game-core::enemy —— 敌方域产品层（two-layer-architecture.md §1：struct + method，零地址字面量）。
//!
//! 命名纪律（AGENTS §3）：语义命名经 data/naming 台账（A 级地址自证 / B 级语义概要级）；
//! 未语义考证字段保留 `_todo_` 前缀 + 触达证据注——禁伪语义命名，宁缺勿滥。
//! F2CA 族九入口 = 本域方法集素材（decode-sweep r1 §3；切片考证=单元 spec 输入）。

pub mod common;
pub mod movement;
pub mod slot;
pub mod state;

pub use movement::{MoveCheckOutcome, MoveCheckStaging};
pub use slot::EnemySlot;
pub mod crawler;

/// waver 模块（TDD 流程（测试先行（实现后补）））——refs asm 考证：bank01:97D7（EnemyMovementChoice_Waver_BANK1（prg1_brinstar.asm:498））（Waver 例程（实址（A 级原文）））

/// mellow 模块（TDD 流程（测试先行（实现后补）））——refs asm 考证：bank07:EE86（SpawnMellow）+bank07:EE63（SpawnMellows）（Mellow 例程（实址（A 级原文）））
pub mod ai_table;
pub(crate) mod anim_idx;
pub(crate) mod anim_tick;
pub(crate) mod explosion;
pub mod framework;
pub mod motion;
pub mod pipe_bug_hole;
pub mod prep;
pub mod sidehopper;
pub mod skree;
pub mod slot_cycle;
pub mod tables;

pub(crate) mod cannon;
pub(crate) mod dragon;
pub(crate) mod en_projectile;
pub(crate) mod kraid;
pub(crate) mod metroid;
pub(crate) mod mother_brain;
pub(crate) mod multiviola;
pub(crate) mod pipe_bug;
pub(crate) mod polyp;
pub(crate) mod ridley;
pub(crate) mod rinka;
pub(crate) mod rio;
pub(crate) mod squeept;
pub(crate) mod swooper;
pub(crate) mod waver;
pub(crate) mod zebetite;
