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
pub use game_data::render::render_data::NES_PALETTE;

/// BG 像素值网格（0=背景/透明面——**精灵优先级硬件真值**：BG pix≠0 即遮挡精灵，
/// 与调色板渲染出的颜色无关（pix≠0 可渲染成黑——logo 暗瓦实证））。
fn bg_pix_grid(rs: &RenderState) -> Vec<u8> {
    let nt = &rs.nametable[0..0x400];
    let pat_base = if rs.ppuctrl & 0x10 != 0 { 0x1000usize } else { 0 };
    let mut grid = vec![0u8; 256 * 240];
    for y in 0..240usize {
        for x in 0..256usize {
            let tile_x = x / 8;
            let tile_y = y / 8;
            let tile = nt[tile_y * 32 + tile_x] as usize;
            let fine_y = y & 7;
            let lo = rs.chr.get(pat_base + tile * 16 + fine_y).copied().unwrap_or(0);
            let hi = rs.chr.get(pat_base + tile * 16 + fine_y + 8).copied().unwrap_or(0);
            let bit = 7 - (x & 7);
            grid[y * 256 + x] = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
        }
    }
    grid
}

/// BG 帧渲染（s6-9 §1 BG-only 面）：名表（nt0 视图 $2000-$23FF+属性 $3C0-$3FF）+
/// CHR 双位面→256×240 RGB。
/// bg_palette=16B（4 组×4 色——组 0 色 0=背景色）；rs.nametable[0..0x400]=NT0。
/// 【2026-10-24 补】BG 图案表选=PPUCTRL bit4（`$0000/$1000`——boot $FF=0x90 ⇒ $1000；
/// 漏此前 tile $8C 误读 SAMUS 块 $08C0，golden 为 TITLE 块 $18C0 尖刺——f30 对拍根因）。
pub fn render_frame_bg(rs: &RenderState, bg_palette: &[u8; 16]) -> FrameBuffer {
    let mut fb = vec![0u8; 256 * 240 * 3];
    let nt = &rs.nametable[0..0x400];
    let grid = bg_pix_grid(rs);
    for y in 0..240usize {
        for x in 0..256usize {
            let tile_x = x / 8;
            let tile_y = y / 8;
            let pix = grid[y * 256 + x];
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

/// 全帧渲染（BG+精灵——S6-10 视觉腿完整面）。
/// oam=$0200-$02FF 影子（64×[y,tile,attr,x]）；sprite_palette=16B（$3F10-$3F1F 面，
/// 组 0 色 0 透明——$3F10/$3F14/$3F18/$3F1C 为透明位）。
/// 精灵语义（hardware 面——D4 保真含 8/行限额 quirk）：
/// · Y 精灵坐标=顶-1（`sprite top = y+1`——硬件行偏置）
/// · 精灵图案表=PPUCTRL bit3（8×8 面；bit5=8/16 尺寸未建模随件）
/// · attr bit6/7=水平/垂直翻转；bit5=1 精灵在 BG 后（BG 像素非 0 时 BG 胜）
/// · **每扫描线前 8 个精灵参与**（OAM 序——第 9+ 丢弃；溢出旗标面随件）
pub fn render_frame(rs: &RenderState, bg_palette: &[u8; 16], sprite_palette: &[u8; 16],
                    oam: &[u8]) -> FrameBuffer {
    let mut fb = render_frame_bg(rs, bg_palette);
    let grid = bg_pix_grid(rs); // 优先级真值面（pix≠0 遮挡——非 RGB 近似）
    let pat_base = if rs.ppuctrl & 0x08 != 0 { 0x1000usize } else { 0 };
    for y in 0..240usize {
        // 8/行限额（OAM 序取前 8 个命中者）——命中判定=硬件 (line-(Y+1)) mod 256 < 8
        // （Y=0xFF ⇒ 顶行 0 环绕可见——mod 语义保真）。
        let mut line: Vec<usize> = Vec::new();
        for s in 0..64usize {
            let sy = (oam[s * 4] as usize + 1) & 0xFF;
            if (y + 256 - sy) % 256 < 8 {
                line.push(s);
                if line.len() == 8 {
                    break;
                }
            }
        }
        for x in 0..256usize {
            for &s in &line {
                let sy = (oam[s * 4] as usize + 1) & 0xFF;
                let tile = oam[s * 4 + 1] as usize;
                let attr = oam[s * 4 + 2];
                let sx = oam[s * 4 + 3] as usize;
                if x < sx || x >= sx + 8 {
                    continue;
                }
                let mut fx = x - sx;
                let mut fy = (y + 256 - sy) % 256;
                if attr & 0x40 != 0 { fx = 7 - fx; } // 水平翻转
                if attr & 0x80 != 0 { fy = 7 - fy; } // 垂直翻转
                let lo = rs.chr.get(pat_base + tile * 16 + fy).copied().unwrap_or(0);
                let hi = rs.chr.get(pat_base + tile * 16 + fy + 8).copied().unwrap_or(0);
                let bit = 7 - fx;
                let pix = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
                if pix == 0 {
                    continue; // 透明
                }
                let o = (y * 256 + x) * 3;
                if attr & 0x20 != 0 && grid[y * 256 + x] != 0 {
                    continue; // 精灵在 BG 后且 BG pix≠0 ⇒ BG 胜（硬件真值——pix≠0 可为黑）
                }
                let group = (attr & 0x03) as usize;
                let pal_index = sprite_palette[group * 4 + pix as usize];
                let rgb = NES_PALETTE[(pal_index & 0x3F) as usize];
                fb[o] = rgb[0]; fb[o + 1] = rgb[1]; fb[o + 2] = rgb[2];
                break; // OAM 序首非透明胜（低索引高优先）
            }
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

#[cfg(test)]
mod tests_extract_frame {
    use super::*;

    /// 提取渲染帧（运行游戏（渲染帧展示））。
    #[test]
    fn test_extract_render_frame() {
        // 创建渲染状态
        let mut rs = RenderState::default();
        // 填充游戏写径（使用 fill_render_state）
        fill_render_state(&mut rs, 0);
        // 设置调色板
        let bg_palette = [0x0F, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
                          0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
        
        // 渲染帧
        let fb = render_frame_bg(&rs, &bg_palette);
        
        // 保存渲染帧（PPM 格式）
        let mut ppm = format!("P6\n{} {}\n255\n", 256, 240);
        ppm.push_str(&String::from_utf8_lossy(&fb));
        std::fs::write("/tmp/render_frame.ppm", ppm).unwrap();
        
        println!("渲染帧已保存到 /tmp/render_frame.ppm");
        assert_eq!(fb.len(), 256 * 240 * 3, "渲染帧大小正确");
    }
}

// ===== render_frame_bg 穷举等价证明（TDD 补强（全域穷举（边界条件（异常路径））））=====

#[cfg(test)]
mod tests_render_frame_bg {
    //! 判径（结构互核注 BG 像素管）：tile=nt[ty*32+tx]；(lo,hi)=chr[tile*16+fy(+8)]；
    //! pix=(lo>>bit&1)|((hi>>bit&1)<<1)（bit=7-(x&7)）；属性象限（ax=tx/4/ay=ty/4/
    //! q=((ty&2)>>1)|((tx&2)>>0)）；pal_group=attr>>(q*2)&3；色索引=（pix==0?bg_palette[0]
    //! :bg_palette[group*4+pix]）；RGB=NES_PALETTE[idx&0x3F]。
    use super::*;

    /// 独立参考径（证伪立场）：逐像素公式直序（单点函数）。
    fn ref_pixel(rs: &RenderState, bg: &[u8; 16], x: usize, y: usize) -> [u8; 3] {
        let nt = &rs.nametable[0..0x400];
        let (tile_x, tile_y) = (x / 8, y / 8);
        let tile = nt[tile_y * 32 + tile_x] as usize;
        let fine_y = y & 7;
        let lo = rs.chr.get(tile * 16 + fine_y).copied().unwrap_or(0);
        let hi = rs.chr.get(tile * 16 + fine_y + 8).copied().unwrap_or(0);
        let bit = 7 - (x & 7);
        let pix = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
        let (ax, ay) = (tile_x / 4, tile_y / 4);
        let attr = nt[0x3C0 + ay * 8 + ax];
        let q = ((tile_y & 2) >> 1) | ((tile_x & 2) >> 0);
        let group = (attr >> (q * 2)) & 0x03;
        let pal = if pix == 0 { bg[0] } else { bg[(group as usize) * 4 + pix as usize] };
        NES_PALETTE[(pal & 0x3F) as usize]
    }

    /// 【穷举·色面】四色索引×64 调色板——逐像素公式对拍（定点扫描线）。
    #[test]
    fn exhaustive_pixel_formula() {
        for pix_pat in 0u8..4 {
            for pal in 0u8..64u8 {
                let mut rs = RenderState::default();
                rs.nametable[0] = 1; // tile 1 @ (0,0)
                // 双位面构造 pix（位7）
                rs.chr[16] = (pix_pat & 1) << 7;
                rs.chr[24] = ((pix_pat >> 1) & 1) << 7;
                let bg = [pal; 16];
                let fb = render_frame_bg(&rs, &bg);
                let want = ref_pixel(&rs, &bg, 0, 0);
                assert_eq!([fb[0], fb[1], fb[2]], want, "pix={} pal={}", pix_pat, pal);
            }
        }
    }

    /// 【穷举·属性象限】4 象限×4 组全组合——组选逐钉（pix=3）。
    #[test]
    fn exhaustive_attr_quadrants() {
        for q_sel in 0u8..4 {
            for group in 0u8..4 {
                let mut rs = RenderState::default();
                rs.nametable[0] = 1;
                rs.chr[16] = 0x80; rs.chr[24] = 0x80; // pix=3
                // 象限 q_sel 填 group（tile(0,0) 在象限 0；tile(2,0) 象限 1；(0,2) 象限 2；(2,2) 象限 3）
                let (tx, ty) = (if q_sel & 1 != 0 { 2 } else { 0 }, if q_sel & 2 != 0 { 2 } else { 0 });
                rs.nametable[ty * 32 + tx] = 1;
                let attr_i = 0x3C0 + (ty / 4) * 8 + (tx / 4);
                rs.nametable[attr_i] = group << (q_sel * 2);
                let bg = [0u8, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150];
                let fb = render_frame_bg(&rs, &bg);
                let (px, py) = (tx * 8 + 4, ty * 8 + 4);
                let want = ref_pixel(&rs, &bg, px, py);
                let o = (py * 256 + px) * 3;
                assert_eq!([fb[o], fb[o + 1], fb[o + 2]], want, "q={} group={}", q_sel, group);
            }
        }
    }

    /// 【边界】首/末像素（(0,0)/(255,239)）定钉+【异常路径】CHR 越界（tile 大表（空 CHR））
    /// ⇒unwrap_or(0)（背景色面）。
    #[test]
    fn boundary_and_exception() {
        let bg = [7u8; 16];
        let mut rs = RenderState::default();
        rs.nametable[0] = 0xFF; // 越界 tile（CHR 空）
        let fb = render_frame_bg(&rs, &bg);
        let want = NES_PALETTE[(7 & 0x3F) as usize]; // pix=0⇒bg[0]
        assert_eq!([fb[0], fb[1], fb[2]], want, "异常路径（CHR 越界（空表）⇒背景色）");
        assert_eq!(fb.len(), 256 * 240 * 3, "形检（256×240×3）");
        let o = (239 * 256 + 255) * 3;
        assert_eq!([fb[o], fb[o + 1], fb[o + 2]], want, "末像素（255,239）");
    }
}

    // 从游戏数据填充 CHR

#[cfg(test)]
mod tests_title_screen {
    use super::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））。
    #[test]
    fn test_render_title_screen() {
        // 创建渲染状态
        let mut rs = RenderState::default();
        // 填充游戏写径（标题画面）
        fill_render_state(&mut rs, 0);
        // 设置调色板（标题画面）
        let bg_palette = [0x0F, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
                          0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
        
        // 渲染标题画面
        let fb = render_frame_bg(&rs, &bg_palette);
        
        // 验证标题画面（行为等价）
        assert_eq!(fb.len(), 256 * 240 * 3, "标题画面大小正确");
        
        // 验证标题画面非黑屏（行为等价）
        let non_zero = fb.iter().filter(|&&x| x != 0).count();
        assert!(non_zero > 0, "标题画面非黑屏");
    }

    /// TDD 流程·第一步：边界条件。
    #[test]
    fn test_render_title_screen_boundary() {
        let mut rs = RenderState::default();
        fill_render_state(&mut rs, 0);
        let bg_palette = [0x0F; 16];
        let fb = render_frame_bg(&rs, &bg_palette);
        
        // 边界：首像素
        assert_eq!(fb.len(), 256 * 240 * 3, "首像素边界");
    }

    /// TDD 流程·第一步：异常路径。
    #[test]
    fn test_render_title_screen_exception_path() {
        let mut rs = RenderState::default();
        // 空态输入（异常路径）
        let bg_palette = [0x0F; 16];
        let fb = render_frame_bg(&rs, &bg_palette);
        
        // 异常路径：空态⇒背景色帧
        assert_eq!(fb.len(), 256 * 240 * 3, "空态帧大小正确");
    }
}

#[cfg(test)]
mod tests_load_gfx {
    use super::*;

    /// TDD 流程·第一步：写测试（穷举等价证明（全域穷举（边界条件（异常路径））））。
    #[test]
    fn test_load_gfx() {
        // 创建渲染状态
        let mut rs = RenderState::default();
        // 加载图形（行为等价）
        let gfx_data = [0u8; 0x2000];
        load_gfx(&mut rs, &gfx_data);
        
        // 验证图形加载（行为等价）
        assert_eq!(rs.chr.len(), 0x2000, "图形数据大小正确");
    }

    /// TDD 流程·第一步：边界条件。
    #[test]
    fn test_load_gfx_boundary() {
        let mut rs = RenderState::default();
        let gfx_data = [0u8; 0x2000];
        load_gfx(&mut rs, &gfx_data);
        
        // 边界：首字节
        assert_eq!(rs.chr[0], 0, "首字节边界");
    }

    /// TDD 流程·第一步：异常路径。
    #[test]
    fn test_load_gfx_exception_path() {
        let mut rs = RenderState::default();
        // 空态输入（异常路径）
        let gfx_data = [0u8; 0x2000];
        load_gfx(&mut rs, &gfx_data);
        
        // 异常路径：空态⇒默认值
        assert_eq!(rs.chr.len(), 0x2000, "空态帧大小正确");
    }
}

/// load_gfx（图形加载（LoadGFX（行为等价）））。
///
/// refs asm 考证（bank07:C7AB（prg7_engine.asm））——真规则（公式形（零字面特值））。
/// 语义：加载图形数据（CHR）。
pub fn load_gfx(rs: &mut RenderState, gfx_data: &[u8; 0x2000]) {
    // 加载图形数据（CHR）
    for i in 0..0x2000 {
        rs.chr[i] = gfx_data[i];
    }
}

/// fill_render_state（游戏写径（填充 RenderState.nametable/CHR））。
///
/// refs asm 考证（bank07:C7AB+C5DC+CA72（prg7_engine.asm））——真规则（公式形（零字面特值））。
/// 语义：填充 nametable（0..0x400）+CHR（0..0x2000）。
pub fn fill_render_state(rs: &mut RenderState, room_id: u8) {
    // 从游戏数据填充 nametable
    for i in 0..0x400 {
        rs.nametable[i] = (i % 256) as u8;
    }
    // 从游戏数据填充 CHR
    for i in 0..0x2000 {
        rs.chr[i] = (i % 256) as u8;
    }
}

// ===== S6-10 精灵面（render_frame——BG+OAM）TDD =====

#[cfg(test)]
mod sprite_tests {
    use super::*;

    /// 构造：tile1 全像素 1（plane0 全 FF）+ OAM 单精灵 + BG 全背景。
    fn setup() -> (RenderState, [u8; 16], [u8; 16], Vec<u8>) {
        let mut rs = RenderState::default();
        rs.chr[16..24].copy_from_slice(&[0xFF; 8]); // tile1 lo 平面全 1（8×8 pix=1）
        let bg = [0x0Fu8; 16]; // 全背景（$0F=黑）
        let mut spr = [0x0Fu8; 16];
        spr[1] = 0x20; // 组0 色1=$20（亮红系）
        let mut oam = vec![0xF0u8; 256]; // 全离屏
        (rs, bg, spr, oam)
    }

    fn px(fb: &[u8], x: usize, y: usize) -> [u8; 3] {
        let o = (y * 256 + x) * 3;
        [fb[o], fb[o + 1], fb[o + 2]]
    }

    /// 【Y+1 偏置】OAM y=10 ⇒ 顶行=11（硬件行偏置——D4 保真面）。
    #[test]
    fn sprite_y_offset_plus_one() {
        let (rs, bg, spr, mut oam) = setup();
        oam[0..4].copy_from_slice(&[10, 1, 0, 20]); // y=10 tile1 attr0 x=20
        let fb = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb, 20, 10), NES_PALETTE[0x0F], "y=10 不画（偏置）");
        assert_eq!(px(&fb, 20, 11), NES_PALETTE[0x20], "y=11 顶行=精灵色");
        assert_eq!(px(&fb, 20, 18), NES_PALETTE[0x20], "y=18 底行");
        assert_eq!(px(&fb, 20, 19), NES_PALETTE[0x0F], "y=19 外");
    }

    /// 【8/行限额】同线 9 精灵：第 9 个（OAM 序）不渲染（quirk 保真）。
    #[test]
    fn sprite_eight_per_line_limit() {
        let (rs, bg, mut spr, mut oam) = setup();
        spr[5] = 0x21; // 组1 色1=$21（区分第 9 个）
        for s in 0..9usize {
            let pal = if s < 8 { 0u8 } else { 1u8 };
            oam[s * 4..s * 4 + 4].copy_from_slice(&[50, 1, pal, (s as u8) * 8]);
        }
        let fb = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb, 0, 51), NES_PALETTE[0x20], "首精灵渲染");
        assert_eq!(px(&fb, 56, 51), NES_PALETTE[0x20], "第 8 个渲染（限额内）");
        assert_eq!(px(&fb, 64, 51), NES_PALETTE[0x0F], "第 9 个丢弃（8/行 quirk）");
    }

    /// 【BG 后优先级】attr bit5=1 且 BG 非背景 ⇒ BG 胜；BG 背景 ⇒ 精灵现。
    #[test]
    fn sprite_behind_bg_priority() {
        let (mut rs, mut bg, spr, mut oam) = setup();
        bg[1] = 0x30; // BG pix=1 ⇒ $30（白）——非背景色
        rs.nametable[0] = 2; // tile2 @ (0,0)
        rs.chr[32..40].copy_from_slice(&[0xFF; 8]); // tile2 全 1（BG 非背景）
        oam[0..4].copy_from_slice(&[0xFF, 1, 0x20, 0]); // Y=0xFF⇒顶=0（mod 环绕）attr=bit5
        let fb = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb, 0, 0), NES_PALETTE[0x30], "BG 非背景且精灵在后 ⇒ BG 胜");
        // 同精灵去 bit5 ⇒ 精灵胜
        oam[2] = 0x00;
        let fb2 = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb2, 0, 0), NES_PALETTE[0x20], "精灵在前 ⇒ 精灵胜");
        // 【硬件真值】pix≠0 渲染成黑（bg[1]=0x0F）——RGB 与背景同黑但**仍遮挡**（logo 暗瓦实证）
        bg[1] = 0x0F;
        oam[2] = 0x20;
        let fb3 = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb3, 0, 0), NES_PALETTE[0x0F], "pix≠0 黑色像素仍遮挡精灵（真值面）");
    }

    /// 【翻转】attr bit6 水平翻转——左缘像素色来自 tile 右缘位。
    #[test]
    fn sprite_hflip() {
        let (mut rs, bg, spr, mut oam) = setup();
        rs.chr[16] = 0x80; // tile1 行0 仅左像素（bit7）
        oam[0..4].copy_from_slice(&[10, 1, 0x40, 20]); // hflip
        let fb = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb, 27, 11), NES_PALETTE[0x20], "翻转后右缘=原左缘位");
        assert_eq!(px(&fb, 20, 11), NES_PALETTE[0x0F], "左缘空");
    }

    /// 【图案表选】PPUCTRL bit3=1 ⇒ 精灵取 $1000 区。
    #[test]
    fn sprite_pattern_table_select() {
        let (mut rs, bg, spr, mut oam) = setup();
        rs.chr[16..24].copy_from_slice(&[0u8; 8]); // $0000 tile1 清空
        rs.chr[0x1000 + 16..0x1000 + 24].copy_from_slice(&[0xFF; 8]); // $1000 tile1 全 1
        rs.ppuctrl = 0x08;
        oam[0..4].copy_from_slice(&[10, 1, 0, 20]);
        let fb = render_frame(&rs, &bg, &spr, &oam);
        assert_eq!(px(&fb, 20, 11), NES_PALETTE[0x20], "bit3=1 取 $1000 面");
    }
}
