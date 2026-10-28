//! `game-core::engine::update_enemy_force_speed_towards_samus` —— asm 例程族（首例程 UpdateEnemy_ForceSpeedTowardsSamus（07:F6B9）（P2 批 1 二次拆（一族一文件（文件名=m1disasm 标签（设计稿 §2））））。

use super::*;

// ---- SC-S2-26（S2-56）员转写（3 员——ForceSpeedTowardsSamus 族；L2 真 0 留白）----

/// $F6B9 UpdateEnemy_ForceSpeedTowardsSamus（root——簇共根形五十）：$82←0+
/// $F74B 调（朝向计算）+$6AF4[x] cmp 2 判（位1 段）+$040D[x] dec。
/// 返回 (写事件, 朝向值)。
pub fn s2_force_speed(x: u8, v6af4_x: u8, facing: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = vec![(0x0082, 0)];
    if v6af4_x == 2 && (facing & 2) == 0 {
        return (ev, facing); // beq 回卷段
    }
    ev.push((0x040D + x as u16, facing)); // dec 段（净效面注）
    (ev, facing)
}

/// $F3CF 员体：ForceSpeed+$F75B/$F682/$F676 链（S1-01 族接面）+尾跳 $F40A。
pub fn s2_f3cf_body(x: u8, v6af4_x: u8, facing: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_force_speed(x, v6af4_x, facing);
    (ev, 0xF40A)
}

/// $F401 员体：ForceSpeed+$F75B/$F51E/$F536 链（S2-26 同族接面）+尾跳 $95E5。
pub fn s2_f401_body(x: u8, v6af4_x: u8, facing: u8) -> (Vec<(u16, u8)>, u16) {
    let (ev, _) = s2_force_speed(x, v6af4_x, facing);
    (ev, 0x95E5)
}

#[cfg(test)]
mod s2_056_tests {
    use super::*;

    /// 三员转写形（ForceSpeed root+二壳）。
    #[test]
    fn s2_056_units_three() {
        let (ev, f) = s2_force_speed(2, 5, 1);
        assert_eq!(f, 1);
        assert!(ev.contains(&(0x0082, 0)));
        assert!(ev.contains(&(0x040F, 1))); // x=2 ⇒ $040D+2
        let (ev2, _) = s2_force_speed(2, 2, 0); // beq 回卷段
        assert_eq!(ev2.len(), 1);
        assert_eq!(s2_f3cf_body(2, 5, 1).1, 0xF40A);
        assert_eq!(s2_f401_body(2, 5, 1).1, 0x95E5);
    }
}

