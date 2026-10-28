//! `game-core::engine::check_bomb_launch` —— asm 例程族（首例程 CheckBombLaunch（07:D150）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S3-15（S3-75）员转写（2 员——$D150 态检查族；L2 真 0 留白）----

/// $D150 员体（root——簇共根形六九）：$6878 lsr/bcs 段+$12|$16 asl/bpl 段+
/// $0308|$0307 判（bne 段——三判链）。
/// 返回 (判段通过, carry)。
pub fn s3_d150_check(v6878: u8, v12: u8, v16: u8, v0308: u8, v0307: u8) -> (bool, bool) {
    let c = v6878 & 1 != 0;
    if !c {
        return (false, false); // bcc 段
    }
    let v = (v12 | v16) << 1;
    if (v & 0x80) != 0 {
        return (false, true); // bmi 段（bpl 未取）
    }
    ((v0308 | v0307) == 0, true) // 尾面（测案-1 (true,true) 对）
}

/// $D136 员体：$D150+$14 位0/1 双判（$CFB7 条件调）+尾跳 $CD6D（A=2）+$0308/$0312 清。
pub fn s3_d136_body(v6878: u8, v12: u8, v16: u8, v0308: u8, v0307: u8, v14: u8)
    -> (Vec<(u16, u8)>, u16) {
    let (ok, _) = s3_d150_check(v6878, v12, v16, v0308, v0307);
    let mut ev = vec![(0x0308, 0), (0x0312, 0)];
    if ok && (v14 & 3) == 0 {
        ev.push((0x0305, 7)); // 条件调面注（写族随链）
    }
    (ev, 0xCD6D)
}

#[cfg(test)]
mod s3_075_tests {
    use super::*;

    /// 二员转写形（$D150 三判链 root+一壳）。
    #[test]
    fn s3_075_units_two() {
        assert_eq!(s3_d150_check(1, 0x10, 0x10, 0, 0), (true, true));
        assert_eq!(s3_d150_check(0, 0x10, 0x10, 0, 0), (false, false)); // bcc 段
        assert_eq!(s3_d150_check(1, 0xC0, 0, 0, 0), (false, true)); // bmi 段（0xC0<<1=0x80 负）
        let (ev, j) = s3_d136_body(1, 0x10, 0x10, 5, 0, 0);
        assert_eq!(j, 0xCD6D);
        assert!(ev.contains(&(0x0308, 0)));
    }
}

