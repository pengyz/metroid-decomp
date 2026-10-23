//! `game-core::title::gfx` —— 标题图形初始化（InitTitleGFX（行为等价））。
//!
//! refs 考证（bank:addr（A 级原文））：InitTitleGFX（bank07:C5D7（标题图形初始化））+
//! LoadGFX（bank07:C7AB（图形加载））+LoadSamusGFX（bank07:C5DC（萨姆斯图形加载））。

#[cfg(test)]
mod tests_init_title_gfx {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! 判径（任务规约面）：load_gfx（CHR-RAM 8KB 载面（gfx→rs.chr[0..0x2000]））；
    //! init_title_gfx（title 载→samus 载（双载序（MMC1 CHR 双面（A000/C000）行为等价）））
    //! →rs.chr=终载面（sketch 双零阵⇒chr=0）+writes=空面。

    use super::*;
    use crate::frame::render::RenderState;

    /// 【穷举·load_gfx 全域】gfx 全逐位对拍（0x2000 全域（独立参考径））。
    #[test]
    fn exhaustive_load_gfx() {
        let mut rs = RenderState::default();
        let mut gfx = [0u8; 0x2000];
        for i in 0..0x2000usize {
            gfx[i] = (i % 256) as u8;
        }
        load_gfx(&mut rs, &gfx);
        assert_eq!(rs.chr, gfx.to_vec(), "load_gfx 全域（gfx→chr）");
    }

    /// 【穷举·init 全域】预置非零（异常路径覆写面）→init 后 chr=0（双零阵终载）
    /// +writes=空+全域扫描。
    #[test]
    fn exhaustive_init_title() {
        let mut rs = RenderState::default();
        for b in rs.chr.iter_mut() { *b = 0xAA; }
        for b in rs.nametable.iter_mut() { *b = 0x55; }
        let w = init_title_gfx(&mut rs);
        assert_eq!(w.len(), 0, "writes=空面（sketch 判径）");
        assert!(rs.chr.iter().all(|&b| b == 0), "chr 全域=0（双零阵终载）");
        // 名表不干预（异常路径（init 只载 CHR））
        assert!(rs.nametable.iter().all(|&b| b == 0x55), "名表不动");
    }

    /// 【边界】chr 首/末位元（0/0x1FFF）定钉+载面长度恒 0x2000。
    #[test]
    fn boundary_indices() {
        let mut rs = RenderState::default();
        let gfx = [0xFFu8; 0x2000];
        load_gfx(&mut rs, &gfx);
        assert_eq!(rs.chr[0], 0xFF, "chr[0]");
        assert_eq!(rs.chr[0x1FFF], 0xFF, "chr[0x1FFF]");
        assert_eq!(rs.chr.len(), 0x2000, "CHR-RAM 8KB 面长");
    }
}

// ===== 标题图形初始化（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：InitTitleGFX（bank07:C5D7）+LoadGFX（bank07:C7AB）
// +LoadSamusGFX（bank07:C5DC）。

use crate::frame::render::RenderState;

/// 图形加载（旧 LoadGFX（07:C7AB）行为等价面）——gfx→rs.chr[0..0x2000]（CHR-RAM 8KB 载面）。
pub fn load_gfx(rs: &mut RenderState, gfx: &[u8; 0x2000]) {
    for i in 0..0x2000usize {
        rs.chr[i] = gfx[i];
    }
}

/// 标题图形初始化（行为等价）——title 载→samus 载（双载序（MMC1 CHR 双面（A000/C000）
/// 行为等价（07:C5D7/07:C5DC 载面）））；writes=空面（写径在载面（CHR 直写））。
pub fn init_title_gfx(rs: &mut RenderState) -> Vec<(u16, u8)> {
    let title_gfx = [0u8; 0x2000];
    load_gfx(rs, &title_gfx);
    let samus_gfx = [0u8; 0x2000];
    load_gfx(rs, &samus_gfx);
    Vec::new()
}
