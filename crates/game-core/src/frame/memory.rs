//! 全内存模型（task-10②）：RAM 2KB（含栈页 $0100-$01FF）+ WRAM 8KB + 寄存器面。
//!
//! 地址归一口径同 L2 契约（2KB 镜像 `addr & 0x7FF`；WRAM $6000-$7FFF 字节偏移）；
//! watch 面=零页 $0000-$00FF + 栈/游戏变量区 $0100-$07FF（l3_manifest 显式基址区间，
//! gotcha-stack-page 教训：不用惯常分区假设）。

/// G11 类型下沉（r19）：定义归 `crate::state`（frame↔域件环消除）——本件保留 re-export
/// （兼容径）+方法实现（同 crate 跨模块 impl）。
pub use crate::state::{FrameMemory, Registers};

impl FrameMemory {
    /// RAM 读（2KB 镜像归一）。
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.ram[(addr as usize) & 0x7FF]
    }

    /// RAM 写（2KB 镜像归一）。
    pub fn write_ram(&mut self, addr: u16, v: u8) {
        self.ram[(addr as usize) & 0x7FF] = v;
    }

    /// WRAM 读（$6000-$7FFF 字节偏移面）。
    pub fn read_wram(&self, addr: u16) -> u8 {
        self.wram[(addr as usize - 0x6000) & 0x1FFF]
    }

    /// WRAM 写。
    pub fn write_wram(&mut self, addr: u16, v: u8) {
        self.wram[(addr as usize - 0x6000) & 0x1FFF] = v;
    }

    /// IO/寄存器面写（PPU/APU/手柄——**不入 RAM**：write_ram 的 2KB 镜像归一会把
    /// $2000 等寄存器址漏进 RAM $0000——本方法是寄存器面唯一入口；渲染面建模挂点=task-13）。
    pub fn write_io(&mut self, addr: u16, v: u8) {
        match addr {
            0x2000 => {
                self.nmi_enabled = v & 0x80 != 0; // PPUCTRL bit7=NMI（hardware.asm:28）
                self.render.ppuctrl = v;          // 全寄存器面（图案表选 bit3/4——渲染腿）
            }
            0x2005 => self.render.write_reg_2005(v),   // PPUSCROLL（V1 摄像机——双写锁存）
            0x2006 => self.render.write_reg_2006(v),   // PPUADDR flip-flop（S6-10 接线）
            0x2007 => {
                // PPUDATA→RenderState 喂入（S6-10 ①——840ad00 建模件运行时喂入面）
                // 增量位=PPUCTRL bit2（1/32）——从 ram $FF 读
                let inc = if self.ram[0x00FF] & 0x04 != 0 { 32u16 } else { 1u16 };
                self.render.write_ppudata(v, inc);
            }
            _ => {
                // APU 面（$4000-$4017）：壳层音频消费源（apu_log 捕获——驱动逐帧 drain）
                if (0x4000..=0x4017).contains(&addr) {
                    self.apu_log.push((addr, v));
                }
            } // 其余寄存器面 S5 骨架暂不建模
        }
        // 其余寄存器面 S5 骨架暂不建模（RAM watch 不可见）；渲染面挂 task-13
    }

    /// 迭代内帧等待（**WaitNMIPass/WriteAndWait 族**（asm prg7:1108-1140 `ClearNMIStat`
    /// +spin 等下一 NMI）——1 tick 等待+NMI 内块全跑（含 ReadJoyPads）。定谳件
    /// findings/move-drift-iteration-scheduling.md R2）。
    pub fn iter_wait(&mut self) {
        self.iter_gates.push(1);
    }

    /// 迭代重活 **bogged 帧**（NMIStatus 跳过门（asm prg7:251 `lda NMIStatus/bne`）
    /// ——内块跳过（无 ReadJoyPads/palette/VRAM）=原版掉帧窗（D4 保真项）。R3 件。
    pub fn iter_bogged(&mut self, n: u8) {
        for _ in 0..n {
            self.iter_gates.push(0);
        }
    }

    /// 臂内帧判定（**Step 3 谓词单源**——engine 缝抑制与 mainloop 续段共用同一实现，
    /// 判定用 tick 首态=上一拍分派遗留）：同相位续段（cursor>0 且归属相位==当前相位
    /// ——F4 孤儿语义）｜完成拍哨兵（0xFF，Step 4——显式排除出续段：完成拍无 prelude
    /// 无分发）｜spin 等待段（r23）。臂内帧=asm 例程体内自旋/续段/完成拍——无 C0D3
    /// 执行 ⇒ 无缝行、无 prelude（map-v2 §4-④/F10——与 span 三性质同源）。
    pub fn title_continuation(&self) -> bool {
        let c = self.title_cursor;
        (c > 0 && c != 0xFF
            && self.title_cursor_phase == self.read_ram(crate::ram_map::TITLE_ROUTINE))
            || c == 0xFF
            || self.title_spin > 0
    }

    /// watch 状态向量（zp/iram/wram/pw，大写 hex——与 oracle 帧 trace 同形）。
    /// **编码唯一源=memmap::frame_face**（严重-6 对账闭合：废本层手写切片第二源）。
    /// 委托化=task-22 slot①（lead 例外授权：只动委托结构不动语义）；wram 全 8KB 钉死（严重-5）。
    /// pw=密码字节面（$6988-$698F——sc-item-pickup 系 manifest watch）。
    pub fn watch_snapshot(&self) -> Vec<(&'static str, String)> {
        vec![
            ("zp", crate::memmap::frame_face::encode_zp(&self.ram)),
            ("iram", crate::memmap::frame_face::encode_iram(&self.ram)),
            ("wram", crate::memmap::frame_face::encode_wram(&self.wram)),
            ("pw", crate::memmap::frame_face::encode_pw(&self.wram)),
        ]
    }
}


    /// 【S6-10 接线】write_io PPU 面喂入 RenderState（$2006 flip-flop+$2007 分域）。
    #[test]
    fn write_io_feeds_render_state() {
        let mut m = FrameMemory::default();
        m.write_io(0x2006, 0x10); // hi
        m.write_io(0x2006, 0x00); // lo → $1000
        m.write_io(0x2007, 0xAB); // CHR 写
        assert_eq!(m.render.chr[0x1000], 0xAB);
        m.write_io(0x2006, 0x20);
        m.write_io(0x2006, 0x00); // $2000
        m.ram[0x00FF] = 0x04; // 32 增量
        m.write_io(0x2007, 0xCD); // 名表写+32 步进
        assert_eq!(m.render.nametable[0], 0xCD);
        assert_eq!(m.render.ppuaddr, 0x2020);
    }
#[cfg(test)]
mod tests {
    use super::*;

    /// 寄存器面不漏 RAM 镜像（write_ram 的 &0x7FF 归一会把 $2000 漏进 $0000——
    /// tick_frame 测试实抓回归案；寄存器面唯一入口=write_io）。
    #[test]
    fn write_io_does_not_leak_into_ram_mirror() {
        let mut m = FrameMemory::default();
        m.write_io(0x2000, 0x90);
        m.write_io(0x2005, 0x00);
        assert_eq!(m.read_ram(0x0000), 0x00);
        assert_eq!(m.read_ram(0x0005), 0x00);
    }

    /// 2KB 镜像归一 + watch 面编码（zp=512 hex、iram=3584 hex，大写）。
    #[test]
    fn ram_mirror_and_watch_face() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0800, 0xAB); // 镜像回 $0000
        assert_eq!(m.read_ram(0x0000), 0xAB);
        assert_eq!(m.read_ram(0x1000), 0xAB); // $1000 镜像面
        let snap = m.watch_snapshot();
        assert_eq!(snap[0].0, "zp");
        assert_eq!(snap[0].1.len(), 512);
        assert_eq!(snap[1].0, "iram");
        assert_eq!(snap[1].1.len(), 3584);
        assert!(snap[0].1.starts_with("AB"));
        assert!(snap[0].1.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }
}

/// MSS↔FrameMemory 映射（**spec §4 分工件**——docs/spec/state-bundle-schema.md §4
/// 接缝表；双腿任意起点收官缝）：
/// · RAM 区（$0000-$07FF）→ ram[0x800] **直映**（A 级）
/// · WRAM（$6000-$7FFF）→ wram[0x2000] **直映**
/// · CPU 寄存器（cpu.a/x/y/sp/pc/ps）→ regs 直映（ps↔p=status 同义名）
/// · mapper.prgReg/chrReg/prgMode=**memmap 扩展面**（task-14 A9 后遗同面——随接）
/// · ppu.*=S6 渲染腿（bundle v1 不含）；apu/clock=时钟/RNG 对表（task-10 定谳后接）。
impl FrameMemory {
    /// MSS 恢复构造（直映三面）。
    pub fn restore_mss(ram: &[u8], wram: &[u8], regs: Registers) -> Self {
        assert!(ram.len() >= 0x800, "MSS RAM 面不足 $800");
        assert!(wram.len() >= 0x2000, "MSS WRAM 面不足 $2000");
        let mut mem = Self::default();
        mem.ram.copy_from_slice(&ram[..0x800]);
        mem.wram.copy_from_slice(&wram[..0x2000]);
        mem.regs = regs;
        mem.nmi_enabled = mem.ram[0x00FF] & 0x80 != 0;
        mem
    }

    /// 导出 MSS 面（反向——三直映；bundle 生产侧用）。
    pub fn capture_mss(&self) -> (Vec<u8>, Vec<u8>, Registers) {
        (self.ram.to_vec(), self.wram.to_vec(), self.regs)
    }
}

#[cfg(test)]
mod mss_map_tests {
    use super::*;

    /// MSS 映射保真（三直映往返+nmi 推导）。
    #[test]
    fn restore_mss_maps_all_faces() {
        let mut ram = vec![0u8; 0x800];
        ram[0x001A] = 1;
        ram[0x00FF] = 0x90;
        let mut wram = vec![0u8; 0x2000];
        wram[0x1234] = 0xAB;
        let regs = Registers { a: 1, x: 2, y: 3, sp: 4, pc: 0x8000, p: 0x24 }; // MSS ps↔p（status 同义）
        let mem = FrameMemory::restore_mss(&ram, &wram, regs);
        assert_eq!(mem.read_ram(0x001A), 1);
        assert_eq!(mem.read_wram(0x7234), 0xAB);
        assert_eq!((mem.regs.a, mem.regs.pc), (1, 0x8000));
        assert!(mem.nmi_enabled);
        let (r2, w2, regs2) = mem.capture_mss();
        assert_eq!(&r2[..0x800], &ram[..]);
        assert_eq!(&w2[..0x2000], &wram[..]);
        assert_eq!(regs2, regs);
    }
}
