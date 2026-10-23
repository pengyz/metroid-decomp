//! kernel::nmi —— 中断处理（渲染同步+输入处理）。
//! refs 考证（bank:addr（A 级原文））：NMI（bank07:C0D9）。

/// 中断处理（渲染同步+输入处理）——$0FF←1（NMI 标志）/$0FA←0（输入清零）。
pub fn nmi_handler(ram: &mut [u8; 2048]) {
    ram[0x00FF] = 0x01;
    ram[0x00FA] = 0x00;
}
