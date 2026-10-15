//! F2CA 族组合流暂存面（地址只活在本层——$010F/$6E/$6F + ROM 向量常量）。
//!
//! 地址证据（A 级）：bank07:F2FD `8D 0F 01` STA $010F（值=$80，`A9 80` LDA 立即数）；
//! bank07:F309 `85 6E` STA zp $6E / F30E `85 6F` STA zp $6F（值←ROM $95CE/$95CF）。
//! ROM 常量按 window_bank=7 约定烘焙（l2-proof-assumptions.md A9）：$95CE=0x03、$95CF=0x38
//! （prg[0x1D5CE..0x1D5D0]=03 38 自证）；语义未证（C 级）——向量/指针假设。

/// F2ED 流暂存址（栈页）。
pub const SCRATCH_010F: u16 = 0x010F;

/// F2ED 流暂存初值（bank07:F2FD `A9 80` LDA #$80——立即数，语义未证 C 级）。
pub const F2ED_SCRATCH_INIT: u8 = 0x80;

/// zp 向量低位暂存址。
pub const ZP_VEC_LO: u16 = 0x006E;

/// zp 向量高位暂存址。
pub const ZP_VEC_HI: u16 = 0x006F;

/// ROM 向量低位（$95CE，window_bank=7 约定=0x03）。
pub const ROM_VEC_LO: u8 = 0x03;

/// ROM 向量高位（$95CF，window_bank=7 约定=0x38）。
pub const ROM_VEC_HI: u8 = 0x38;

/// 写事件映射（暂存址；同值写也是事件）。
pub fn event_010f(v: u8) -> (u16, u8) {
    (SCRATCH_010F, v)
}

/// 写事件映射（zp 向量低位）。
pub fn event_vec_lo(v: u8) -> (u16, u8) {
    (ZP_VEC_LO, v)
}

/// 写事件映射（zp 向量高位）。
pub fn event_vec_hi(v: u8) -> (u16, u8) {
    (ZP_VEC_HI, v)
}
