//! game-core::render —— S6-8 名表清屏（收编于 frame/render.rs RenderState——单一型纪律）。
//!
//! spec 主文档：docs/spec/rendering-chain.md §2.3（task-42）。
//! 【结构互核注】①名表清屏=ClearNameTables $C158 语义：NT0 恒清+模式分路
//! （GameMode==Play(0)⇒NT1；否则 TitleRoutine==EndGame(0x1D)⇒NT2 否则 NT1）
//! ②ClearNameTable $C175 填充环：4 页×256B 填充（名表+属性整块 $400B）
//! ③镜像映射：NT2/NT3 垂直镜像→NT0/NT1 区（frame/render.rs 2KB 面覆盖 $2000-$27FF）。
//! 【2026-10-18 收编】先版自建 RenderState=与 840ad00 重复（卡图澄清后收编——单一型）。

use crate::frame::render::RenderState;

/// 名表区映射（NT 号→frame/render.rs nametable vec 偏移——垂直镜像 NT2→0/NT3→1）。
fn nt_offset(nt_plus1: u8) -> usize {
    let nt = nt_plus1.wrapping_sub(1) as usize;
    (nt & 1) * 0x400
}

/// ClearNameTable（$C175）：单名表填充环（名表+属性整块 $400B←fill）+
/// RAM 面（$00←fill/$01←NT+1/$FF&=~$04——PPUCTRL INCR_DOWN 清）。
pub fn clear_name_table(rs: &mut RenderState, nt_plus1: u8, fill: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u8) {
    let off = nt_offset(nt_plus1);
    for b in rs.nametable[off..off + 0x400].iter_mut() {
        *b = fill;
    }
    let new_ctrl = ppuctrl & !0x04;
    (vec![(0x0000, fill), (0x0001, nt_plus1), (0x00FF, new_ctrl)], new_ctrl)
}

/// ClearNameTables（$C158）：NT0 恒清+模式分路（Play⇒NT1；EndGame(0x1D)⇒NT2 否则 NT1）。
/// 返回 (RAM 写事件, 新 PPUCTRL)。
pub fn clear_name_tables(rs: &mut RenderState, game_mode: u8, title_routine: u8, ppuctrl: u8)
    -> (Vec<(u16, u8)>, u8) {
    let (mut events, ctrl) = clear_name_table(rs, 1, 0xFF, ppuctrl);
    let nt2 = game_mode != 0 && title_routine == 0x1D;
    let nt_plus1 = if nt2 { 3 } else { 2 };
    let (ev2, ctrl2) = clear_name_table(rs, nt_plus1, 0xFF, ctrl);
    events.extend(ev2);
    (events, ctrl2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 名表清屏双面（NT0 恒清+模式分路+镜像映射）。
    #[test]
    fn clear_name_tables_dispatch() {
        let mut rs = RenderState::default();
        let (ev, ctrl) = clear_name_tables(&mut rs, 0, 0, 0x95);
        // Play⇒NT1：NT0(0x000-0x3FF)+NT1(0x400-0x7FF) 清
        assert!(rs.nametable[0..0x800].iter().all(|b| *b == 0xFF));
        assert!(ev.iter().any(|e| *e == (0x0001, 1)));
        assert!(ev.iter().any(|e| *e == (0x0001, 2)));
        assert_eq!(ctrl, 0x91); // INCR_DOWN 清
        // EndGame(0x1D)⇒NT2（镜像→NT0 区——再次清）
        let mut rs = RenderState::default();
        let (ev, _) = clear_name_tables(&mut rs, 1, 0x1D, 0);
        assert!(ev.iter().any(|e| *e == (0x0001, 3)));
        assert!(rs.nametable[0..0x400].iter().all(|b| *b == 0xFF)); // NT2→NT0 区
    }

    /// 镜像映射（NT2/3→0/1 区）。
    #[test]
    fn nt_mirroring_map() {
        assert_eq!(nt_offset(1), 0);
        assert_eq!(nt_offset(2), 0x400);
        assert_eq!(nt_offset(3), 0); // NT2→NT0 区（垂直镜像）
        assert_eq!(nt_offset(4), 0x400);
    }
}
