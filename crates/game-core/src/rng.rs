//! rng —— 游戏 PRNG 状态推进域（two-layer 产品层：纯函数 + 状态 struct，零地址字面量）。
//!
//! 语义证据（A 级三轨；考证全录 docs/findings/game-rng-adjudication.md）：
//! ① asm 结构互核 bank07:C000-C019（m1disasm `RandomNumbers` 内容锚 × ROM 26B 逐字节）：
//!    循环 ×5 内 RN1 += 0x05、RN2 += 0x13 ⇒ **每调用** RN1 += 0x19、RN2 += 0x5F（mod 256）；
//!    返回 A=RN1 终值（尾 `LDA $2E/RTS`）；X 经栈保存恢复（TXA/PHA/PLA/TAX）；Y 不动。
//!    「every frame」注=**调用聚合注**（第六案「注释≠语义」族正面样本）：动态证 call≠frame
//!    ——boot-title 300 帧仅 295 次调用（标题期跳帧）。
//! ② 种子序 bank07:$C0AC（`A9 11 85 2E A9 FF 85 2F`）：RN1←0x11、RN2←0xFF。
//! ③ 动态交叉（运行态确证，Mesen2 三用途标签）：boot-title 逐帧 zp 快照——295 次调用
//!    转移全=+0x19/+0x5F（增量集单元素）、闭式 0 违例。
//!
//! 闭式（k=调用计数，**非帧号**）：RN1_k=(0x11+0x19k)&0xFF、RN2_k=(0xFF+0x5Fk)&0xFF；
//! 双轴周期各 256（gcd(0x19,256)=gcd(0x5F,256)=1），联合周期 256。

/// 每调用步进量（RN1 轴）：循环内 +0x05 × 5 次——bank07:C004-C00A。
pub const RN1_STEP: u8 = 0x19;

/// 每调用步进量（RN2 轴）：循环内 +0x13 × 5 次——bank07:C00B-C011。
pub const RN2_STEP: u8 = 0x5F;

/// 种子初值（bank07:$C0AC `A9 11 85 2E`）。
pub const RN1_SEED: u8 = 0x11;

/// 种子初值（bank07:$C0AC `A9 FF 85 2F`）。
pub const RN2_SEED: u8 = 0xFF;

/// PRNG 状态面（**全状态=两轴累加器**；面外读写=0 由 L2 写面契约钉死——考证 §4）。
/// 字段名 rn1/rn2=第三方名 RandomNumber1/2 的机械缩写（语义自证=①②③ 三轨，N-0012）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RngState {
    /// RN1 轴（zp 字节域由 memmap::rng_state 承载）。
    pub rn1: u8,
    /// RN2 轴（同上）。
    pub rn2: u8,
}

/// 步进结果（返回值=RN1 终值——C000 尾 `LDA $2E/RTS`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RngStep {
    pub a: u8,
}

impl RngState {
    /// 种子态（bank07:$C0AC 初始化序；动态自证 boot-title f=1 的 zp 快照=0x11/0xFF）。
    pub fn boot_seed() -> Self {
        Self { rn1: RN1_SEED, rn2: RN2_SEED }
    }

    /// 每调用步进（RandomNumbers 单次调用的域折叠）：两轴各 +步进量（mod 256），
    /// 返回 A=RN1 终值。入参 A 不影响（C000 首条 TXA 即覆盖）；X/Y 面由适配层透传。
    pub fn step(&mut self) -> RngStep {
        self.rn1 = self.rn1.wrapping_add(RN1_STEP);
        self.rn2 = self.rn2.wrapping_add(RN2_STEP);
        RngStep { a: self.rn1 }
    }

    /// 闭式第 k 态（考证 §2 的机器核形式；k=调用计数）。
    /// 周期 256 ⇒ k mod 256 足够（0x19·256 ≡ 0x5F·256 ≡ 0 mod 256）。
    pub fn at_call(k: u32) -> Self {
        let k = (k & 0xFF) as u8;
        Self {
            rn1: RN1_SEED.wrapping_add(RN1_STEP.wrapping_mul(k)),
            rn2: RN2_SEED.wrapping_add(RN2_STEP.wrapping_mul(k)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bank07:C000 单次调用语义：种子态 → 0x2A/0x5E（+0x19/+0x5F），A=RN1 终值。
    #[test]
    fn step_advances_both_axes_and_returns_rn1() {
        let mut st = RngState::boot_seed();
        let out = st.step();
        assert_eq!((st.rn1, st.rn2), (0x2A, 0x5E));
        assert_eq!(out.a, 0x2A);
    }

    /// 闭式 ↔ 逐调用步进同一性（归纳钉死：k=0..300 全同）。
    #[test]
    fn closed_form_matches_repeated_steps() {
        let mut st = RngState::boot_seed();
        for k in 0..300u32 {
            assert_eq!(st, RngState::at_call(k), "k={k}");
            st.step();
        }
    }

    /// 双轴周期 256（at_call(256)==种子态；动态 295 调用内 0x11 态未复现=周期下界佐证）。
    #[test]
    fn period_is_256_on_both_axes() {
        assert_eq!(RngState::at_call(256), RngState::boot_seed());
        assert_eq!(RngState::at_call(0), RngState::boot_seed());
    }

    /// 8 位回绕边界（L2 案 b-game-rng-wrap-* 同源语义）。
    #[test]
    fn step_wraps_at_byte_boundary() {
        let mut st = RngState { rn1: 0xE7, rn2: 0xAB };
        let out = st.step();
        assert_eq!((st.rn1, st.rn2, out.a), (0x00, 0x0A, 0x00));
    }
}
