//! `game-core::samus::animation` —— 萨姆斯动画（设置+帧更新面（TDD 件））。
//!
//! refs 考证（bank:addr（A 级原文））：SetSamusAnim（bank07:CF6B（萨姆斯动画设置））+
//! SetSamusData_3FrameAnimDelay（bank07:CD6B（3 帧动画延迟））+LoadSamusGFX（bank07:C5DC
//! （萨姆斯图形加载））。

#[cfg(test)]
mod tests_samus_animation {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：writes=[$0305←A/$0306←A/$0304←0]，ret=0（A 归零面）。

    use super::*;

    /// 独立参考径（证伪立场）：三写直序+零出。
    fn reference_set_anim(a: u8) -> (Vec<(u16, u8)>, u8) {
        (vec![(0x0305, a), (0x0306, a), (0x0304, 0)], 0u8)
    }

    /// 【穷举·a_in 全域】0..=255 逐位对拍（写序/写值/零出）。
    #[test]
    fn exhaustive_anim_face() {
        for a in 0u8..=255 {
            assert_eq!(set_samus_animation(a), reference_set_anim(a), "a_in={}", a);
        }
    }

    /// 【边界】极端值（0x00/0x7F/0x80/0xFF）+异常路径（写面长度恒 3/ret 恒 0/次序恒定）。
    #[test]
    fn boundary_and_exception() {
        for a in [0u8, 0x7F, 0x80, 0xFF] {
            let (ev, r) = set_samus_animation(a);
            assert_eq!(ev.len(), 3, "写面长度恒 3");
            assert_eq!(r, 0, "ret 恒 0");
            assert_eq!(ev[0], (0x0305, a), "次序 0（$0305）");
            assert_eq!(ev[1], (0x0306, a), "次序 1（$0306）");
            assert_eq!(ev[2], (0x0304, 0), "次序 2（$0304←0）");
        }
    }
}

// ===== 萨姆斯动画（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：SetSamusAnim（bank07:CF6B）+
// SetSamusData_3FrameAnimDelay（bank07:CD6B）+LoadSamusGFX（bank07:C5DC）。

/// 萨姆斯动画设置——writes=[$0305←A/$0306←A/$0304←0]（SetSamusAnim 写面（07:CF6B）
/// +动画索引对写）；ret=0（A 归零面）。3 帧延迟面（CD6B）留扩展缝（判径保留）。
pub fn set_samus_animation(a_in: u8) -> (Vec<(u16, u8)>, u8) {
    let mut ev = Vec::with_capacity(3);
    ev.push((0x0305, a_in));
    ev.push((0x0306, a_in));
    ev.push((0x0304, 0));
    (ev, 0)
}
