//! CD9C 探针折叠（F2CA 族窗口内联子例程；two-layer：地址+折叠逻辑住映射层）。
//!
//! 折叠定性经 lead 认可（纯标志位例程：无写集、Y←$0300-1 经调用方 PLA/TAY 恢复、
//! **进位为关键输出**）。地址证据（A 级）：bank07:CD9C-CDBE 全区
//! `38 AC 00 03 88 D0 1B AD 78 68 29 08 F0 14 AD 05 03 C9 0E F0 0A C9 0C 38
//! D0 08 2C 08 03 10 03 CD 06 03 60`（逐字节自证）。
//! 注：L2 输出面无 flags——进位只作窗内分支输入（F2ED 流 BCC $F310）。

/// CD9C 探针结果（A=RTS 时刻累加器；carry=出口进位）。
pub struct Cd9cProbe {
    pub a: u8,
    pub carry: bool,
}

/// 折叠执行 CD9C 语义（只读探针，无写集）。
pub fn probe(ram: &[u8], wram: &[u8], a_in: u8) -> Cd9cProbe {
    let rd = |a: u16| -> u8 { ram[a as usize & 0x7FF] };
    let wr = |a: u16| -> u8 { wram.get(a as usize - 0x6000).copied().unwrap_or(0) };
    // CD9C: SEC / LDY $0300 / DEY / BNE exit（C=1，A 未动）
    if rd(0x0300).wrapping_sub(1) != 0 {
        return Cd9cProbe { a: a_in, carry: true };
    }
    // LDA $6878 / AND #$08 / BEQ exit（C=1，A=与结果）
    let a1 = wr(0x6878) & 0x08;
    if a1 == 0 {
        return Cd9cProbe { a: 0, carry: true };
    }
    // LDA $0305 / CMP #$0E / BEQ CDBB
    let a2 = rd(0x0305);
    if a2 == 0x0E {
        return Cd9cProbe { a: a2, carry: a2 >= rd(0x0306) };
    }
    // CMP #$0C / SEC / BNE exit（SEC 后 C=1）
    if a2 != 0x0C {
        return Cd9cProbe { a: a2, carry: true };
    }
    // BIT $0308 / BPL exit（C=1）
    if rd(0x0308) & 0x80 == 0 {
        return Cd9cProbe { a: a2, carry: true };
    }
    // CDBB: CMP $0306
    Cd9cProbe { a: a2, carry: a2 >= rd(0x0306) }
}
