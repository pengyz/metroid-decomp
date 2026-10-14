//! game-oracle —— L2 差分宿主（6502 核直跑 ROM 原字节；D7 机制①的裁定引擎）。
//!
//! 三方全等：`case.out(golden) == run_subroutine(ROM) == game_core::l2ports`。
//! 本 crate 不持有判定门禁；gates/cli.py subroutine-diff 经 tools/diff_subroutines.py 调用。

pub mod cpu6502;
pub mod l2;

pub use l2::{run_subroutine, L2RunResult};
