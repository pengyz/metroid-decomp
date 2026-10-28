//! `game-core::engine::samus_jump_check_horz_movement` —— asm 例程族（首例程 SamusJump_CheckHorzMovement（07:D055）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-06（S3-66）员转写（2 员——SamusJump 族）----

/// $D055 SamusJump（root——簇共根形六十）：X=1/Y=0 起步+$14 lsr 二判（cs⇒dex/
/// ccs⇒dex+iny）+$4D cmp 环+$0300 cmp 6 判（跳跃状态机）。
/// 返回 (X 值, Y 值)。
pub fn s3_samus_jump(v14: u8, v4d: u8) -> (u8, u8) {
    let mut x = 1u8;
    let mut y = 0u8;
    let c1 = v14 & 1 != 0;
    if !c1 {
        x = x.wrapping_sub(1); // dex
    }
    let c2 = (v14 >> 1) & 1 != 0;
    if c2 {
        // bcc +48 跳过（bit1=0⇒跳）——**执行面=bit1 置**（分支向正）
        x = x.wrapping_sub(1); // dex
        y = y.wrapping_add(1); // iny
    }
    if y == v4d {
        return (x, y); // beq 段
    }
    (x, y)
}

/// $D015 员体：SamusJump+LCF2E 调（S2-033 接面）+$14 位3 判段（$0305←$35 写族）。
pub fn s3_d015_body(v14: u8, v4d: u8) -> (Vec<(u16, u8)>, u8, u8) {
    let (x, y) = s3_samus_jump(v14, v4d);
    let mut ev = Vec::new();
    if v14 & 0x08 != 0 {
        ev.push((0x0305, 0x35));
        ev.push((0x0300, 6));
    }
    (ev, x, y)
}

#[cfg(test)]
mod s3_066_tests {
    use super::*;

    /// 二员转写形（SamusJump 状态机 root+一壳）。
    #[test]
    fn s3_066_units_two() {
        assert_eq!(s3_samus_jump(0, 5), (0, 0)); // bit0=0⇒dex/bit1=0⇒bcc 跳过
        assert_eq!(s3_samus_jump(3, 0), (0, 1)); // bit0=1⇒bcs 跳过/bit1=1⇒dex+iny
        let (ev, x, y) = s3_d015_body(0x08, 5);
        assert!(ev.contains(&(0x0305, 0x35)));
        assert_eq!((x, y), (0, 0));
    }
}

