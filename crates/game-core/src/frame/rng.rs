//! GameRng——游戏 PRNG（**算法未考证**：不发明，AGENTS §3）。
//!
//! 对齐 oracle 的算法待执行序引爆定位（zp RNG 面 + 取值例程 bank:addr）后按 rename 流程
//! 落账；未证前为 fail-loud 桩——首次消费即爆（形态与 unported! 缺口面一致）。

/// 游戏 PRNG 句柄（算法缺口面）。
#[derive(Clone, Debug, Default)]
pub struct GameRng;

impl GameRng {
    /// 新建（种子语义随算法考证落定）。
    pub fn new() -> Self {
        Self
    }

    /// 下一字节（算法未证——fail-loud）。
    pub fn next_u8(&mut self) -> u8 {
        crate::unported!("rng-algorithm-unverified（待执行序引爆定位 bank:addr）")
    }
}
