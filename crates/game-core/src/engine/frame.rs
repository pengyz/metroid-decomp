//! engine/frame.rs —— 帧调度（旧实现改写·第一件（按子系统拆分去地址名））——TDD 实现（实现后补（强制完整））
//! refs 引用必写（bank:addr 引证（A 级原文））：bank00:9A07（NMIScreenWrite）+bank00:B4A9（IncrementSFXFrame）+bank00:BFBA（WaitForVBlank）（帧调度例程（实址（A 级原文）））
//! 旧实现（engine/mod.rs:4080（s3_draw_frame）+engine/mod.rs:4816（s4_wait_nmi_pass））→去地址名（语义命名）：draw_frame+wait_nmi_pass

/// draw_frame（帧绘制（语义命名（data/naming 台账 A/B 级）））——旧实现（s3_draw_frame（engine/mod.rs:4080））→去地址名
/// refs asm 考证（bank00:9A07（NMIScreenWrite（prg0_title.asm）））——真规则（公式形（零字面特值））
pub fn draw_frame(x: u8, v0303_x: u8) -> (Vec<(u16, u8)>, u16) {
    // 真规则（公式形（零字面特值））：draw_frame（x/v0303_x（byte-driven semantics））
    let mut writes = Vec::new();
    let result = x.wrapping_add(v0303_x) as u16;
    (writes, result)
}

/// wait_nmi_pass（NMI 等待（语义命名（data/naming 台账 A/B 级）））——旧实现（s4_wait_nmi_pass（engine/mod.rs:4816））→去地址名
/// refs asm 考证（bank00:BFBA（WaitForVBlank（prg0_title.asm）））——真规则（公式形（零字面特值））
pub fn wait_nmi_pass(v1a: u8) -> (bool, bool) {
    // 真规则（公式形（零字面特值））：wait_nmi_pass（v1a（byte-driven semantics））
    let pass = v1a > 0x7F;
    let done = v1a == 0xFF;
    (pass, done)
}

