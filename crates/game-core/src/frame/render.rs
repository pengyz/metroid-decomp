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
}

impl Default for RenderState {
    fn default() -> Self {
        Self { chr: vec![0; 0x2000], nametable: vec![0; 0x800], ppuaddr: 0, latch_hi: true }
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
            _ => {} // 调色板域（$3F00）等——随件
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
