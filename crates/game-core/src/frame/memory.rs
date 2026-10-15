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
}

impl Default for FrameMemory {
    fn default() -> Self {
        Self { ram: [0; 0x800], wram: [0; 0x2000], regs: Registers::default(), nmi_enabled: false }
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

    /// watch 状态向量（zp/iram，大写 hex——与 oracle 帧 trace 同形）。
    pub fn watch_snapshot(&self) -> Vec<(&'static str, String)> {
        vec![
            ("zp", hex_upper(&self.ram[0x000..0x100])),
            ("iram", hex_upper(&self.ram[0x100..0x800])),
        ]
    }
}

/// 大写 hex 编码（watch 面）。
fn hex_upper(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02X}");
    }
    s
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
