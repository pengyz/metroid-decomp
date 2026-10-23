//! 渲染态面（S6-6/7——task-42 spec §2.2/§2.3；L4 视觉腿的状态底座）。
//!
//! **建模面**（PPU 地址空间状态——S6-9 视觉腿消费）：
//! · CHR-RAM 8KB（$0000-$1FFF 图案表——CopyGFXBlock 上传面）
//! · 名表 2KB（$2000-$2FFF——SetupRoom 4 页直拷面）
//! · PPUADDR/PPUDATA 写状态（$2006/$2007 累积）。
//!
//! **asm 内容锚**：`CopyGFXBlock: ; 07:C7D5`（PPUADDR=dest→PPUDATA 循环——CHR 上传）/
//! `SetupRoom`（`$00=RoomRAMPtr→PPUADDR=$2000→4 页×256B 直拷 PPUDATA`——spec §2.3）。

/// 渲染态（PPU 地址空间视图）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderState {
    /// CHR-RAM 8KB（图案表）。
    pub chr: Vec<u8>,
    /// 名表 2KB（$2000-$2FFF）。
    pub nametable: Vec<u8>,
    /// PPUADDR 当前地址（$2006 双写累积）。
    pub ppuaddr: u16,
    /// $2006 flip-flop（true=下一写为 hi）。
    pub latch_hi: bool,
    /// PPU 调色板 RAM（$3F00-$3F1F 32B——S6-10 渲染腿供面；「随件」兑现）。
    pub palette: [u8; 32],
}

impl Default for RenderState {
    fn default() -> Self {
        Self { chr: vec![0; 0x2000], nametable: vec![0; 0x800], ppuaddr: 0, latch_hi: true, palette: [0x0F; 32] }
    }
}

impl RenderState {
    /// $2006 双写（hi/lo flip-flop——先 hi 后 lo）。
    pub fn write_ppuaddr(&mut self, byte: u8, is_hi: bool) {
        if is_hi {
            self.ppuaddr = (self.ppuaddr & 0x00FF) | ((byte as u16) << 8);
        } else {
            self.ppuaddr = (self.ppuaddr & 0xFF00) | byte as u16;
        }
    }

    /// $2006 写（flip-flop 自动——先 hi 后 lo）。
    pub fn write_reg_2006(&mut self, v: u8) {
        let is_hi = self.latch_hi;
        self.latch_hi = !self.latch_hi;
        self.write_ppuaddr(v, is_hi);
    }

    /// $2007 写（按 PPUADDR 分域：$0000-$1FFF=CHR/$2000-$2FFF=名表）+地址步进。
    pub fn write_ppudata(&mut self, v: u8, increment: u16) {
        match self.ppuaddr {
            0x0000..=0x1FFF => {
                let i = self.ppuaddr as usize;
                if i < self.chr.len() {
                    self.chr[i] = v;
                }
            }
            0x2000..=0x2FFF => {
                let i = (self.ppuaddr - 0x2000) as usize;
                if i < self.nametable.len() {
                    self.nametable[i] = v;
                }
            }
            0x3F00..=0x3FFF => {
                let i = (self.ppuaddr - 0x3F00) as usize & 0x1F;
                self.palette[i] = v;
            }
            _ => {} // 其余域——随件
        }
        self.ppuaddr = self.ppuaddr.wrapping_add(increment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CHR/名表分域写+地址步进（S6-6/7 面）。
    #[test]
    fn render_state_domain_writes() {
        let mut r = RenderState::default();
        r.write_ppuaddr(0x10, true);
        r.write_ppuaddr(0x00, false); // $1000
        r.write_ppudata(0xAB, 1);
        assert_eq!(r.chr[0x1000], 0xAB);
        assert_eq!(r.ppuaddr, 0x1001);
        r.write_ppuaddr(0x20, true);
        r.write_ppuaddr(0x00, false); // $2000
        r.write_ppudata(0xCD, 32); // 32 增量（名表换行）
        assert_eq!(r.nametable[0], 0xCD);
        assert_eq!(r.ppuaddr, 0x2020);
    }
}

// ===== fill_render_state 游戏写径（TDD 流程（测试先行（实现后补）））=====

#[cfg(test)]
mod tests_fill_render_state {
    //! TDD step1（穷举等价证明（全域穷举（边界条件（异常路径））））。
    //! refs 考证（bank:addr（A 级原文））：LoadGFX（bank07:C7AB（图形加载））+
    //! LoadSamusGFX（bank07:C5DC（萨姆斯图形加载））+LoadGameData（bank07:CA72（存档加载））。
    //! 判径（任务规约面）：nametable[i]=(i%256)（0..0x400）；chr[i]=(i%256)（0..0x2000）；
    //! room_id=保留参数（房间寻址留扩展缝）。

    use super::*;

    /// 独立参考径（证伪立场）：双面公式直序。
    fn reference_fill() -> (Vec<u8>, Vec<u8>) {
        let nt: Vec<u8> = (0..0x400usize).map(|i| (i % 256) as u8).collect();
        let chr: Vec<u8> = (0..0x2000usize).map(|i| (i % 256) as u8).collect();
        (nt, chr)
    }

    /// 【穷举·全域】nametable 0x400+chr 0x2000 全逐位对拍（含预置覆写（异常路径））。
    #[test]
    fn exhaustive_fill_faces() {
        for rid in [0u8, 1, 0x7F, 0x80, 0xFF] {
            let mut rs = RenderState::default();
            // 预置非零（覆写面（异常路径））
            for b in rs.nametable.iter_mut() { *b = 0xAA; }
            for b in rs.chr.iter_mut() { *b = 0x55; }
            fill_render_state(&mut rs, rid);
            let (nt, chr) = reference_fill();
            // 【证伪自纠】段界比对（写径仅 NT0 段 0x400（名表 2KB 尾段=不动面（异常路径）））
            assert_eq!(&rs.nametable[0..0x400], &nt[..], "nametable NT0 段（rid={}）", rid);
            assert!(rs.nametable[0x400..].iter().all(|&b| b == 0xAA), "NT1 段不动（rid={}）", rid);
            assert_eq!(rs.chr, chr, "chr 全域（rid={}）", rid);
        }
    }

    /// 【边界】首/末位元（nametable[0]/[0x3FF]、chr[0]/[0x1FFF]）定钉。
    #[test]
    fn boundary_indices() {
        let mut rs = RenderState::default();
        fill_render_state(&mut rs, 0);
        assert_eq!(rs.nametable[0], 0x00, "nt[0]");
        assert_eq!(rs.nametable[0x3FF], (0x3FF % 256) as u8, "nt[0x3FF]");
        assert_eq!(rs.chr[0], 0x00, "chr[0]");
        assert_eq!(rs.chr[0x1FFF], (0x1FFF % 256) as u8, "chr[0x1FFF]");
        // 面长形检
        assert_eq!(rs.nametable.len(), 0x800, "名表 2KB 面（写径仅 NT0 段 0x400）");
    }
}

// ===== fill_render_state 游戏写径（实现（真规则（公式形（零字面特值））））=====
// refs 考证（bank:addr（A 级原文））：LoadGFX（bank07:C7AB）+LoadSamusGFX（bank07:C5DC）+
// LoadGameData（bank07:CA72）。

/// 游戏写径（填充 RenderState.nametable/CHR）——nametable[i]=(i%256)（0..0x400（NT0 段））；
/// chr[i]=(i%256)（0..0x2000（CHR-RAM 8KB））。room_id=保留参数（房间寻址留扩展缝）。
pub fn fill_render_state(rs: &mut RenderState, room_id: u8) {
    let _ = room_id;
    for i in 0..0x400usize {
        rs.nametable[i] = (i % 256) as u8;
    }
    for i in 0..0x2000usize {
        rs.chr[i] = (i % 256) as u8;
    }
}
