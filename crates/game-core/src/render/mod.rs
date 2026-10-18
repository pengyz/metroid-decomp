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

// ---- S6-9 视觉腿：帧缓冲产出（BG-only 先行）----
// spec：s6-9-visual-leg.md §1（RenderState→256×240→调色板索引→RGB）。
// 【结构互核注】①BG 像素管：名表 tile→CHR 双位面→2 位像素→属性表象限选调色板组→
// bg_palette[组×4+位]→RGB（64 色 NES 调色板）②属性表=名表 $3C0-$3FF 段（32×32 象限
// 2 位选组）③确定性=纯函数（同输入同帧缓冲逐字节——验收判据 4）。

/// 256×240 RGB 帧缓冲（3B/像素——PNG 8 位 RGB 输入面）。
pub type FrameBuffer = Vec<u8>; // 256*240*3

/// NES 64 色调色板（RGB 三元组烘焙——白名单面参照 spec §3）。
pub const NES_PALETTE: [[u8; 3]; 64] = [
    [0x66,0x66,0x66],[0x00,0x2A,0x88],[0x14,0x12,0xA7],[0x3B,0x00,0xA4],[0x5C,0x00,0x7E],[0x6E,0x00,0x40],[0x6C,0x06,0x00],[0x56,0x1D,0x00],
    [0x33,0x35,0x00],[0x0B,0x48,0x00],[0x00,0x52,0x00],[0x00,0x4F,0x08],[0x00,0x40,0x4D],[0x00,0x00,0x00],[0x00,0x00,0x00],[0x00,0x00,0x00],
    [0xAD,0xAD,0xAD],[0x15,0x5F,0xD9],[0x42,0x40,0xFF],[0x75,0x27,0xFE],[0xA0,0x1A,0xCC],[0xB7,0x1E,0x7B],[0xB5,0x31,0x20],[0x99,0x4E,0x00],
    [0x6B,0x6D,0x00],[0x38,0x87,0x00],[0x0C,0x93,0x00],[0x00,0x8F,0x32],[0x00,0x7C,0x8D],[0x00,0x00,0x00],[0x00,0x00,0x00],[0x00,0x00,0x00],
    [0xFF,0xFE,0xFF],[0x64,0xB0,0xFF],[0x92,0x90,0xFF],[0xC6,0x76,0xFF],[0xF3,0x6A,0xFF],[0xFE,0x6E,0xCC],[0xFE,0x81,0x70],[0xEA,0x9E,0x22],
    [0xBC,0xBE,0x00],[0x88,0xD8,0x00],[0x5C,0xE4,0x30],[0x45,0xE0,0x82],[0x48,0xCD,0xDE],[0x4F,0x4F,0x4F],[0x00,0x00,0x00],[0x00,0x00,0x00],
    [0xFF,0xFE,0xFF],[0xC0,0xDF,0xFF],[0xD3,0xD2,0xFF],[0xE8,0xC8,0xFF],[0xFB,0xC2,0xFF],[0xFE,0xC4,0xEA],[0xFE,0xCC,0xC5],[0xF7,0xD8,0xA5],
    [0xE4,0xE5,0x94],[0xCF,0xEF,0x96],[0xBD,0xF4,0xAB],[0xB3,0xF3,0xCC],[0xB5,0xEB,0xF2],[0xB8,0xB8,0xB8],[0x00,0x00,0x00],[0x00,0x00,0x00],
];

/// BG 帧渲染（s6-9 §1 BG-only 面）：名表（nt0 视图 $2000-$23FF+属性 $3C0-$3FF）+
/// CHR 双位面→256×240 RGB。
/// bg_palette=16B（4 组×4 色——组 0 色 0=背景色）；rs.nametable[0..0x400]=NT0。
pub fn render_frame_bg(rs: &RenderState, bg_palette: &[u8; 16]) -> FrameBuffer {
    let mut fb = vec![0u8; 256 * 240 * 3];
    let nt = &rs.nametable[0..0x400];
    for y in 0..240usize {
        for x in 0..256usize {
            let tile_x = x / 8;
            let tile_y = y / 8;
            let tile = nt[tile_y * 32 + tile_x] as usize;
            let fine_y = y & 7;
            let lo = rs.chr.get(tile * 16 + fine_y).copied().unwrap_or(0);
            let hi = rs.chr.get(tile * 16 + fine_y + 8).copied().unwrap_or(0);
            let bit = 7 - (x & 7);
            let pix = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
            // 属性表象限（32×32——$3C0 段）
            let ax = tile_x / 4;
            let ay = tile_y / 4;
            let attr = nt[0x3C0 + ay * 8 + ax];
            let q = ((tile_y & 2) >> 1) | ((tile_x & 2) >> 0); // 象限位
            let pal_group = (attr >> (q * 2)) & 0x03;
            let pal_index = if pix == 0 {
                bg_palette[0] // 背景色（组 0 色 0）
            } else {
                bg_palette[(pal_group as usize) * 4 + pix as usize]
            };
            let rgb = NES_PALETTE[(pal_index & 0x3F) as usize];
            let o = (y * 256 + x) * 3;
            fb[o] = rgb[0]; fb[o + 1] = rgb[1]; fb[o + 2] = rgb[2];
        }
    }
    fb
}

#[cfg(test)]
mod s69_tests {
    use super::*;

    /// BG 帧渲染确定性（验收判据 4：同输入同帧缓冲逐字节）+形检（256×240×3）。
    #[test]
    fn render_bg_deterministic() {
        let mut rs = RenderState::default();
        rs.nametable[0] = 1; // tile 1 @ (0,0)
        rs.chr[16] = 0x80;   // tile 1 第 0 行 lo 位面（bit7=左像素）
        let pal = [0x0F; 16];
        let a = render_frame_bg(&rs, &pal);
        let b = render_frame_bg(&rs, &pal);
        assert_eq!(a, b, "确定性（判据 4）");
        assert_eq!(a.len(), 256 * 240 * 3);
        // 像素 (0,0)=位 1（lo=1/hi=0）→pal[1]→0x0F→NES_PALETTE[0x0F]=黑
        assert_eq!(&a[0..3], &[0, 0, 0]);
        // 像素 (1,0)=位 0→背景色 pal[0]=0x0F 同黑
        assert_eq!(&a[3..6], &[0, 0, 0]);
    }

    /// 属性表象限选组（32×32 2 位）。
    #[test]
    fn attr_quadrant_select() {
        let mut rs = RenderState::default();
        rs.nametable[0] = 1;
        rs.nametable[3 * 32 + 3] = 1; // tile (3,3)=tile 1（先版漏设——测误正）
        rs.nametable[0x3C0] = 0b11_00_00_00; // 象限 3（右下）组 3
        rs.chr[16] = 0x80; // 位 1
        let mut pal = [0x0F; 16];
        pal[3 * 4 + 1] = 0x21; // 组 3 色 1=0x21
        let fb = render_frame_bg(&rs, &pal);
        // (0,0) 在象限 0（组 0）→pal[1]=0x0F
        assert_eq!(&fb[0..3], &[0, 0, 0]);
        // (24,24) tile (3,3) 在象限 3（组 3）→pal[13]=0x21
        let o = (24 * 256 + 24) * 3;
        let rgb = NES_PALETTE[0x21];
        assert_eq!(&fb[o..o + 3], &rgb);
    }
}
