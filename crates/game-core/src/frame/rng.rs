//! GameRng——游戏 PRNG（**算法未考证**：不发明，AGENTS §3）。
//!
//! 对齐 oracle 的算法待执行序引爆定位（zp RNG 面 + 取值例程 bank:addr）后按 rename 流程
//! 落账；未证前为 fail-loud 桩——首次消费即爆（形态与 unported! 缺口面一致）。

/// 游戏 PRNG 句柄（算法缺口面）。
///
/// 缺口址自证：asm refs/m1disasm/SRC/prg7_engine.asm:33 `RandomNumbers: ; 07:C000`
/// （注：「RandomNumber1 is increased by #$19 every frame / RandomNumber2 … by #$5F」——
/// 累加器族算法，主循环 `jsr RandomNumbers ($C000)` 每帧调用）；算法细节未端口——
/// 爆在精确址（检测面要求 bank:addr 字面量，verifier v4 路由②）。
#[derive(Clone, Debug, Default)]
pub struct GameRng;

impl GameRng {
    /// 新建（种子语义随算法考证落定）。
    pub fn new() -> Self {
        Self
    }

    /// 下一字节（算法未证——fail-loud）。
    pub fn next_u8(&mut self) -> u8 {
        crate::unported!("bank07:C000")
    }
}
