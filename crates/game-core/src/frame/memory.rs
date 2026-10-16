//! 全内存模型（task-10②）：RAM 2KB（含栈页 $0100-$01FF）+ WRAM 8KB + 寄存器面。
//!
//! 地址归一口径同 L2 契约（2KB 镜像 `addr & 0x7FF`；WRAM $6000-$7FFF 字节偏移）；
//! watch 面=零页 $0000-$00FF + 栈/游戏变量区 $0100-$07FF（l3_manifest 显式基址区间，
//! gotcha-stack-page 教训：不用惯常分区假设）。

/// 寄存器面（6502 程序员态 + PC）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub sp: u8,
    pub pc: u16,
}

/// 全内存模型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameMemory {
    /// RAM 2KB 归一镜像面（$0000-$07FF）。
    pub ram: [u8; 0x800],
    /// WRAM 8KB（$6000-$7FFF）。
    pub wram: [u8; 0x2000],
    /// 寄存器面。
    pub regs: Registers,
    /// NMI 使能态（PPUCTRL bit7——write_io($2000) 权威面；60Hz VBlank 调度开关）。
    pub nmi_enabled: bool,
    /// title 子步游标（跨帧件段索引——port2 契约面：next_cursor>0=spin 续点/0=完成复位）。
    pub title_cursor: u8,
    /// 手柄 1 输入面（场景驱动；boot-title=无输入 0）。
    pub joy1: u8,
    /// 手柄 2 输入面。
    pub joy2: u8,
}

impl Default for FrameMemory {
    fn default() -> Self {
        Self { ram: [0; 0x800], wram: [0; 0x2000], regs: Registers::default(), nmi_enabled: false, title_cursor: 0, joy1: 0, joy2: 0 }
    }
}

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
        if addr == 0x2000 {
            self.nmi_enabled = v & 0x80 != 0; // PPUCTRL bit7=NMI 使能（hardware.asm:28）
        }
        // 其余寄存器面 S5 骨架暂不建模（RAM watch 不可见）；渲染面挂 task-13
    }

    /// watch 状态向量（zp/iram/wram，大写 hex——与 oracle 帧 trace 同形）。
    /// **编码唯一源=memmap::frame_face**（严重-6 对账闭合：废本层手写切片第二源）。
    /// 委托化=task-22 slot①（lead 例外授权：只动委托结构不动语义）；wram 全 8KB 钉死（严重-5）。
    pub fn watch_snapshot(&self) -> Vec<(&'static str, String)> {
        vec![
            ("zp", crate::memmap::frame_face::encode_zp(&self.ram)),
            ("iram", crate::memmap::frame_face::encode_iram(&self.ram)),
            ("wram", crate::memmap::frame_face::encode_wram(&self.wram)),
        ]
    }
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
