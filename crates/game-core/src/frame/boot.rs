//! boot 执行序端口（task-10④）：bank07:$C01A 主初始化单元（首引爆点移交件）。
//!
//! 解码证据（A 级，精修 6502 表逐指令；task-12 调用图输入素材——指针留本 doc）：
//! `bank07:C01A` 起 `A9 00 8D 00 A0 ×5 8D 00 C0 ×5 20 FA C4 CA 9A A0 07 … 4C BC C0`。
//! 结构：①MMC1 复位写（CHR0/CHR1 各 5 次+折叠 C4FA 串行 PRG 写——C4FA 零 RAM 写、X 不动，
//! 折叠先例=F1FA/E449）②DEX/TXS 栈指针 ③**RAM 低区清零环**（pages $07..$02，(zp),Y
//! 值=偏移低字节；$00/$01 为指针，环不触 ZP/栈页——帧 1 实测 $0300-$07FF 全零=后续单元
//! 覆写后态，本环为中间态）④**WRAM 清零环**（$7F00 降至 $6000，同型填充）⑤zp 初始化序
//! （$25/$28/$FD/$FC/$1D/…；其中 JSR $C158/$C1A3/$C4B2/$C4DE 为嵌套单元——桩阵逐个引爆）。
//! ⑥主循环（C0BC）+ $1A 握手（NMI 置位）——tick_frame 语义：boot 一次 + 每帧 NMI 单元
//! （bank07:$C0D9，桩）+ 主循环一迭代。
//!
//! 执行序队列指针（task-12/da90f66 交接）：本单元序列与 tools/call_graph.py 执行序队列
//! （seq|bank:addr|调用深度|域|首执行|命中）boot 相 43 入口 f0 reset 链 11 节点对表——
//! 嵌套桩序（C158/C1A3/C4B2/C4DE/C266/C114/C000）即该链的未端口段，逐个引爆=按队列 seq 排。

use super::memory::FrameMemory;

/// MMC1 寄存器面（memmap 纪律：非 RAM 地址只活在此；bank07:$FFB0/$C01A 序引用）。
pub mod io {
    /// MMC1 CHR0（bank07:C01C `8D 00 A0` STA $A000）。
    pub const MMC1_CHR0: u16 = 0xA000;
    /// MMC1 CHR1（bank07:C02B `8D 00 C0` STA $C000）。
    pub const MMC1_CHR1: u16 = 0xC000;
    /// MMC1 PRG（bank07:C4FA 序 `8D 00 E0` STA $E000 ×5 串行）。
    pub const MMC1_PRG: u16 = 0xE000;
    /// PPU 滚动（bank07:C07B `8C 05 20` STY $2005 ×2）。
    pub const PPU_SCROLL: u16 = 0x2005;
    /// PPU 控制（bank07:C08C `8D 00 20` STA $2000，值 $90=NMI 使能）。
    pub const PPU_CTRL: u16 = 0x2000;
    /// APU $4011（bank07:C09E）。
    pub const APU_4011: u16 = 0x4011;
    /// APU $4015（bank07:C0A3）。
    pub const APU_4015: u16 = 0x4015;
}

/// zp 指针对（清零环用；bank07:C041 `84 01`/C045 `84 00`）。
const PTR_LO: u16 = 0x0000;
const PTR_HI: u16 = 0x0001;

/// bank07:$C01A 主初始化（执行序一步一单元；未端口步=unported! 引爆）。
pub fn boot_init(mem: &mut FrameMemory) {
    // ① MMC1 复位写（A=0：CHR0/CHR1 串行清零）
    mem.regs.a = 0x00;
    mmc1_serial_write(mem, io::MMC1_CHR0);
    mmc1_serial_write(mem, io::MMC1_CHR1);
    // 折叠 C4FA（bank07:C4FA `8D 00 E0/4A ×4 … A5 00/60`）：PRG 串行写 A>>n，末值 A=RAM[$00]
    mmc1_serial_write(mem, io::MMC1_PRG);
    mem.regs.a = mem.read_ram(PTR_LO);
    // ② DEX/TXS（X=0（reset 序 LDX #$00）→ $FF）
    mem.regs.x = mem.regs.x.wrapping_sub(1);
    mem.regs.sp = mem.regs.x;
    // ③ RAM 低区清零环（pages $07..$02，值=偏移低字节；不触 ZP/栈页）
    clear_pages(mem, 7, 2);
    // ④ WRAM 清零环（$7F00 降至 $6000）
    clear_pages_wram(mem, 0x7F, 0x60);
    // ⑤ zp 初始化序（bank07:C06D-；$25/$28/$FD/$FC/$1D 后即嵌套单元）
    mem.write_ram(0x0025, 0x0E); // LDA #$0E/STA $25
    mem.write_ram(0x0028, 0x00);
    mem.write_ram(0x00FD, 0x00);
    mem.write_ram(0x00FC, 0x00);
    mem.write_ram(io::PPU_SCROLL, 0x00); // STY $2005 ×2（Y=0）
    mem.write_ram(io::PPU_SCROLL, 0x00);
    mem.write_ram(0x001D, 0x01); // INY/STY $1D
    // 嵌套单元桩（执行序逐个引爆：每引爆一个端一个）
    crate::unported!("bank07:C158");
    #[allow(unreachable_code)]
    {
        // C158 后序（桩移除后可达；现为执行序地图占位）
        mem.write_ram(io::PPU_CTRL, 0x90); // STA $2000（NMI 使能）
        mem.write_ram(0x00FF, 0x90);
        mem.write_ram(0x00FE, 0x02);
        mem.write_ram(0x00FA, 0x47);
        crate::unported!("bank07:C4B2");
        #[allow(unreachable_code)]
        {
            mem.write_ram(io::APU_4011, 0x00);
            mem.write_ram(io::APU_4015, 0x0F);
            mem.write_ram(0x001F, 0x00);
            mem.write_ram(0x001E, 0x00);
            mem.write_ram(0x002E, 0x11);
            mem.write_ram(0x002F, 0xFF);
            mem.write_ram(0x0024, 0x01);
            crate::unported!("bank07:C4DE");
        }
    }
}

/// MMC1 串行写（5 次 STA，值 LSR 递移——bank07:C01C/C02B/C4FA 形态；非 RAM 面）。
fn mmc1_serial_write(mem: &mut FrameMemory, _reg: u16) {
    let mut v = mem.regs.a;
    for _ in 0..5 {
        let _ = v; // STA reg（MMC1 移位面——RAM watch 不可见）
        v >>= 1; // LSR A（C4FA 形态；C01C/C02B 恒写 0 形态同效）
    }
}

/// RAM 低区清零环（bank07:C03F-C055）：pages hi 降至 end_excl（CPX #$01 等值退出），
/// 每页 RAM[base+Y]=Y（值=偏移低字节）；$00/$01 指针环内不被触碰（页≥$02）。
/// 帧界不可见性实测：帧 1 $0300-$07FF 全零=后续单元覆写后态（本环为中间态）。
fn clear_pages(mem: &mut FrameMemory, mut page: u8, end_excl: u8) {
    loop {
        let base = (page as u16) << 8;
        for y in 0..=0xFFu8 {
            mem.write_ram(base + y as u16, y);
        }
        page = page.wrapping_sub(1);
        if page == 0xFF || page < end_excl {
            break;
        }
    }
}

/// WRAM 清零环（bank07:C057-C06B）：pages hi 降至 lo（CPX #$60/BCS 含等值页）。
fn clear_pages_wram(mem: &mut FrameMemory, mut page: u8, lo: u8) {
    loop {
        let base = (page as u16) << 8;
        for y in 0..=0xFFu8 {
            mem.write_wram(base + y as u16, y);
        }
        if page == lo {
            break;
        }
        page = page.wrapping_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 清零环语义（bank07:C03F-C055）：pages $07..$02 值=偏移低字节；ZP/栈页不触。
    #[test]
    fn clear_ram_lo_fills_pages_with_low_byte() {
        let mut m = FrameMemory::default();
        clear_pages(&mut m, 7, 2);
        assert_eq!(m.read_ram(0x0700), 0x00);
        assert_eq!(m.read_ram(0x02FF), 0xFF);
        assert_eq!(m.read_ram(0x0355), 0x55); // 值=偏移低字节
        assert_eq!(m.read_ram(0x0000), 0x00); // ZP 不触
        assert_eq!(m.read_ram(0x0180), 0x00); // 栈页不触
    }

    /// WRAM 环（bank07:C057-C06B）：$6000-$7FFF 同型填充。
    #[test]
    fn clear_wram_fills_with_low_byte() {
        let mut m = FrameMemory::default();
        clear_pages_wram(&mut m, 0x7F, 0x60);
        assert_eq!(m.read_wram(0x6000), 0x00);
        assert_eq!(m.read_wram(0x7FFF), 0xFF);
        assert_eq!(m.read_wram(0x72AB), 0xAB);
    }
}
