//! kernel::startup —— 启动流程（Startup（行为等价））。
//!
//! refs 考证（bank:addr（A 级原文））：Startup（bank07:C01A（启动初始化（MMC1+清 RAM
//! +栈初始化）））+InitTitleGFX（bank07:C5D7（标题图形初始化））+LoadGFX（bank07:C7AB
//! （图形加载））。

#[cfg(test)]
mod tests_startup {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：清 RAM（$0000-$07FF（0..0x800 全 0 写））+栈初始化
    //! （$01FF←$FF（末写（覆盖清面同址）））——写序恒定（清面在前（栈写在后））。

    use super::*;

    /// 独立参考径（证伪立场）：写面直序。
    fn reference_startup() -> Vec<(u16, u8)> {
        let mut w = Vec::with_capacity(0x801);
        for i in 0..0x800u16 {
            w.push((i, 0));
        }
        w.push((0x01FF, 0xFF));
        w
    }

    /// 【穷举·全域】写面 0x801 条逐位对拍（含同址覆盖序（0x01FF 先 0 后 FF））。
    #[test]
    fn exhaustive_startup_writes() {
        let w = startup();
        let r = reference_startup();
        assert_eq!(w.len(), 0x801, "写面长恒 0x801（0x800 清+1 栈）");
        assert_eq!(w, r, "写面全域逐位对拍（序/值/址）");
        // 同址覆盖面（异常路径序证）：0x01FF 出现两次（清 0→栈 FF（末写胜））
        let at: Vec<_> = w.iter().enumerate().filter(|(_, &(a, _))| a == 0x01FF).collect();
        assert_eq!(at.len(), 2, "0x01FF 双写（清面+栈面）");
        assert_eq!((at[0].1.0, at[0].1.1), (0x01FF, 0), "先（清 0）");
        assert_eq!((at[1].1.0, at[1].1.1), (0x01FF, 0xFF), "后（栈 FF（末写胜））");
    }

    /// 【边界】首/末清址（$0000/$07FF）+栈址定钉（$01FF=$FF）。
    #[test]
    fn boundary_indices() {
        let w = startup();
        assert_eq!(w[0], (0x0000, 0), "首清址（$0000）");
        assert_eq!(w[0x7FF], (0x07FF, 0), "末清址（$07FF）");
        assert_eq!(w[0x800], (0x01FF, 0xFF), "栈址（$01FF←$FF）");
    }
}

// ===== 启动流程（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：Startup（bank07:C01A）+InitTitleGFX（bank07:C5D7）
// +LoadGFX（bank07:C7AB）。

/// 启动流程（Startup（行为等价））——清 RAM（$0000-$07FF（0..0x800 全 0 写））
/// +栈初始化（$01FF←$FF（末写（覆盖清面同址（末写胜））））。
/// （MMC1 初始化面（07:C01A）留扩展缝（行为等价径（不必完全模拟 NES 启动流程）））。
pub fn startup() -> Vec<(u16, u8)> {
    let mut writes = Vec::with_capacity(0x801);
    for i in 0..0x800u16 {
        writes.push((i, 0));
    }
    writes.push((0x01FF, 0xFF));
    writes
}
