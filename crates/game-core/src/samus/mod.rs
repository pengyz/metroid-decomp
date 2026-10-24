//! samus 域（task-25；链规 19：**按业务结构收**非 asm 镜像）。
//!
//! 三域：`handler`（状态机脊柱——10 态分发表）/`physics`（积分器+坐标件+R-mv 规则面
//! ——随拼装收编）/`scroll`（相机面——随件收）。
//! spec 源=docs/spec/samus-movement.md（R-mv-1..5 v0.1）——细化随笔落（lead 裁定）。

pub mod handler;
pub mod physics;
pub mod integrators; // 【G6 归位（r19）】原 crate::integrators（int_y/int_x 物理积分件）

pub use handler::{update_samus, SamusHandler};
pub use physics::{vert_accelerate, x_minus_16, x_plus_4};
pub mod animation;
