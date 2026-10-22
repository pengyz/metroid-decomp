//! ram_layout——RAM 布局常量（typed 常量）（数据驱动架构（game-data 纯数据））
//! refs 引用必写：bank:addr 引证（A 级原文）
//! refs asm 考证：constants_ram.asm（RAM 布局（结构布局（写径实证（A 级原文））））

/// SAMUS_Y（RAM 布局（结构布局（写径实证（A 级原文））））——typed 常量
pub const SAMUS_Y: u16 = 0x0300;
/// SAMUS_X（RAM 布局（结构布局（写径实证（A 级原文））））——typed 常量
pub const SAMUS_X: u16 = 0x0302;
/// SAMUS_HEALTH（RAM 布局（结构布局（写径实证（A 级原文））））——typed 常量
pub const SAMUS_HEALTH: u16 = 0x0304;
