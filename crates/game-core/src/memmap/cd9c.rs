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

/// 出口表（P16-F1；**地址=ROM 字节自证**，与复审模型转述的 CDB5/CDBF 差 1——
/// 疑其解码行首计数差；行为等价已由 F2ED/F2E8/F282 全族 L2 案 ROM 裁定钉死）：
///
/// | # | 出口指令 | 目标 | 条件 | 出口 C | 出口 A |
/// |---|---|---|---|---|---|
/// | 1 | CDA1 `D0 1B` BNE | CDBE | ($0300-1)&$FF≠0 | 1（SEC 后未改） | A_in（LDA 未执行） |
/// | 2 | CDA8 `F0 14` BEQ | CDBE | ($6878&8)==0 | 1 | 0（AND 结果） |
/// | 3 | CDB4 `D0 08` BNE | CDBE | $0305≠$0C（前三关过） | 1（**CDB3 SEC 刚置**，覆盖 CMP 借位） | $0305 |
/// | 4 | CDB9 `10 03` BPL | CDBE | $0305==$0C ∧ $0308 bit7=0 | 1（SEC 后 BIT 不改 C） | $0305 |
/// | 5 | CDBB `CD 06 03` CMP 落 CDBE | CDBE | $0305==$0E ∨ ($0305==$0C ∧ $0308 bit7=1) | ($0305>=$0306) | $0305 |
///
/// 注：L2 输出面无 flags——出口 C 只作窗内分支输入；出口 A 观测面见 f2ed（被 PLA 覆盖为
/// Y_in）与 f282（被后续 LDA 覆盖）——A 终值非探针 A 的两处流控已在各自 shim doc 钉死。
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
