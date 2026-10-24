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

    /// 算法步进（纯函数面）：(n1+RN1_STEP, n2+RN2_STEP)——【G9 归一（r19）】步进常量
    /// 单源=crate::rng（RngState 同值面）；本件只留句柄形（D1 接口占位）。
    pub fn step(n1: u8, n2: u8) -> (u8, u8) {
        (n1.wrapping_add(crate::rng::RN1_STEP), n2.wrapping_add(crate::rng::RN2_STEP))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 算法合成值（注×5 互证面）——与 crate::rng::RngState 单源一致（G9 归一钉死）。
    #[test]
    fn game_rng_step_resolved() {
        assert_eq!(GameRng::step(0x11, 0xFF), (0x2A, 0x5E));
        let mut s = crate::rng::RngState { rn1: 0x11, rn2: 0xFF };
        s.step();
        assert_eq!((s.rn1, s.rn2), GameRng::step(0x11, 0xFF), "双面同源（步进常量单源）");
    }
}
