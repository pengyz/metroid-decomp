//! GameRng——游戏 PRNG（**算法已考证**：RandomNumber1/2 双累加器）。
//!
//! asm 引证（A 级）：refs/m1disasm/SRC/prg7_engine.asm `RandomNumbers: ; 07:C000`（注
//! 「RandomNumber1 is increased by #$19 every frame / RandomNumber2 … by #$5F every
//! frame」——体 `ldx #$05` 循环 `+5/+$13`×5=**+$19/+$5F 合成值注体互证**）。
//! 实现走 frame::mainloop::random_numbers（RAM 态面 $2E/$2F）；本 handle 保留为 D1
//! 接口占位。【N-0012 解案】原「算法未证」桩（bank07:C000 fail-loud）已退役——执行序
//! 引爆定位+asm 考证闭环（口径：C 级缺口→引爆定位→asm 自证→解案）。

/// 游戏 PRNG 句柄（算法=双累加器——步进走 mainloop::random_numbers）。
#[derive(Clone, Debug, Default)]
pub struct GameRng;

impl GameRng {
    /// 新建（种子=RAM 态 $2E/$2F 的 boot 初值 $11/$FF——C0AC 自证）。
    pub fn new() -> Self {
        Self
    }

    /// 算法步进（纯函数面）：(n1+$19, n2+$5F)——asm 合成值自证。
    pub fn step(n1: u8, n2: u8) -> (u8, u8) {
        (n1.wrapping_add(0x19), n2.wrapping_add(0x5F))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 算法合成值（注×5 互证面）。
    #[test]
    fn game_rng_step_resolved() {
        assert_eq!(GameRng::step(0x11, 0xFF), (0x2A, 0x5E));
    }
}
