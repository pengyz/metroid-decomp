//! game-core::render —— 渲染输出面（S6 线：RenderState→帧缓冲契约）。
//!
//! spec 主文档：docs/spec/rendering-chain.md（task-42）+docs/spec/s6-9-visual-leg.md
//! （decomp 529f567）。本模块=S6-8 起步（名表清屏建模）——S6-1..7/9 逐卡滚动。
//!
//! 【结构互核注】①名表清屏=ClearNameTables $C158 语义：NT0 恒清+模式分路
//! （GameMode==Play(0)⇒NT1；否则 TitleRoutine==EndGame⇒NT2 否则 NT1）②ClearNameTable
//! $C175 填充环：PPU 复位锁+PPUCTRL_ZP&=~INCR_DOWN（增量=1）+HiPPUTable 表取高址+
//! **4 页×256B 填充**（$FF 面）③属性表=名表后 $3FF-$2FF 段随清（名表+属性整块）。

/// RenderState（S6 面：名表/CHR/PPU 影子——帧缓冲产出源）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderState {
    /// 4 名表（各 0x400B：0x3C0 名表+0x40 属性表）。
    pub nametables: [[u8; 0x400]; 4],
    /// PPUCTRL 影子（$FF）。
    pub ppuctrl: u8,
    /// PPUADDR 影子（地址锁状态——写序建模面）。
    pub ppuaddr: u16,
}

impl Default for RenderState {
    fn default() -> Self {
        Self { nametables: [[0; 0x400]; 4], ppuctrl: 0, ppuaddr: 0 }
    }
}

/// ClearNameTable（$C175）：单名表填充环（4 页×256B——名表+属性整块 $400B←fill）+
/// RAM 面（$00←fill/$01←NT+1/$FF&=~$04）。
/// 返回 RAM 写事件（PPU 副作用=名表填充直反映 RenderState）。
pub fn clear_name_table(rs: &mut RenderState, nt_plus1: u8, fill: u8) -> Vec<(u16, u8)> {
    let nt = (nt_plus1.wrapping_sub(1) as usize) & 3;
    for b in rs.nametables[nt].iter_mut() {
        *b = fill;
    }
    // RAM 面（Temp00_FillValue=$00/Temp01_NameTablePlus1=$01/PPUCTRL_ZP=$FF）
    rs.ppuctrl &= !0x04; // PPUCTRL_INCR_DOWN 清（增量=1）
    vec![(0x0000, fill), (0x0001, nt_plus1), (0x00FF, rs.ppuctrl)]
}

/// ClearNameTables（$C158）：NT0 恒清+模式分路（Play(0)⇒NT1；EndGame(0x1D)⇒NT2 否则 NT1）。
/// 返回 RAM 写事件（两轮）。
pub fn clear_name_tables(rs: &mut RenderState, game_mode: u8, title_routine: u8) -> Vec<(u16, u8)> {
    let mut events = clear_name_table(rs, 1, 0xFF); // NT0（NT+1=1）
    let nt2 = if game_mode != 0 { title_routine == 0x1D } else { false };
    let nt_plus1 = if nt2 { 3 } else { 2 };
    events.extend(clear_name_table(rs, nt_plus1, 0xFF));
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 名表清屏双面（NT0 恒清+模式分路）。
    #[test]
    fn clear_name_tables_dispatch() {
        let mut rs = RenderState::default();
        let ev = clear_name_tables(&mut rs, 0, 0); // Play 模式⇒NT1
        assert!(rs.nametables[0].iter().all(|b| *b == 0xFF));
        assert!(rs.nametables[1].iter().all(|b| *b == 0xFF));
        assert!(rs.nametables[2].iter().all(|b| *b == 0x00));
        assert!(ev.contains(&(0x0001, 1)));
        assert!(ev.contains(&(0x0001, 2)));
        // EndGame（0x1D）⇒NT2
        let mut rs = RenderState::default();
        clear_name_tables(&mut rs, 1, 0x1D);
        assert!(rs.nametables[2].iter().all(|b| *b == 0xFF));
        assert!(rs.nametables[1].iter().all(|b| *b == 0x00));
    }

    /// 单名表填充（含 PPUCTRL INCR_DOWN 清）。
    #[test]
    fn clear_single_name_table() {
        let mut rs = RenderState { ppuctrl: 0x95, ..Default::default() };
        let ev = clear_name_table(&mut rs, 3, 0xFF);
        assert!(rs.nametables[2].iter().all(|b| *b == 0xFF));
        assert_eq!(rs.ppuctrl, 0x91); // 0x95 & ~0x04
        assert!(ev.contains(&(0x00FF, 0x91)));
    }
}
