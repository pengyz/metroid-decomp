//! Object 槽视图（$0300 块）触达子集 ↔ RAM 字节域（地址只活在本层——two-layer §1）。
//!
//! 地址证据（A 级）：bank07:FEB1 `6D 01 03` ADC $0301 / FEB8 `6D 02 03` ADC $0302
//! （FE83 读 Samus.radiusY/radiusX）；bank07:F186-F199 `BD 0D 03`/`BD 0E 03`/`BD 0C 03`
//! （槽位置 Y/X/hi，索引寻址）。
//! 结构证据：refs/m1disasm/SRC/constants_ram.asm:217 `.struct Object`（status@0、
//! radiusY@$0301、radiusX@$0302、isHit@$030A、posY@$030D、posX@$030E、hi@$030C 族）。
//! **视图范围=FE83/F186 触达子集**；全 struct 视图（speed/anim 族）留待批量期（缺口注，
//! 不发明未触达字段的语义）。isHit 面复用 [`crate::memmap::enemy_attrs`]（双语境：
//! task-8 命名=敌属性掩码数组；m1disasm 语境=Object.isHit「Samus hit by enemy」——
//! 同字节多语境正是映射层职责，台账 N-0065 注记）。

/// Object 块基址（Samus=槽 0；敌槽=+0x10 步进，X 索引面）。
pub const OBJECT_BASE: u16 = 0x0300;

/// radiusY 字段址（Object @1；「Distance in pixels from object center to top or bottom」）。
pub const RADIUS_Y: u16 = 0x0301;

/// radiusX 字段址（Object @2）。
pub const RADIUS_X: u16 = 0x0302;

/// 槽 hi 字段偏移（@0x0C；nametable 位）。
pub const OFF_HI: u16 = 0x0C;

/// 槽 posY 字段偏移（@0x0D）。
pub const OFF_POS_Y: u16 = 0x0D;

/// 槽 posX 字段偏移（@0x0E）。
pub const OFF_POS_X: u16 = 0x0E;

/// Samus.radiusY 读（槽 0 固定——FE83 `adc Samus.radiusY` 绝对寻址）。
pub fn read_radius_y(ram: &[u8]) -> u8 {
    ram[RADIUS_Y as usize & 0x7FF]
}

/// Samus.radiusX 读。
pub fn read_radius_x(ram: &[u8]) -> u8 {
    ram[RADIUS_X as usize & 0x7FF]
}

/// 槽 posY 读（F186 `$30D+Y` 面）。
pub fn read_pos_y(ram: &[u8], y: u8) -> u8 {
    ram[(OBJECT_BASE + OFF_POS_Y + y as u16) as usize & 0x7FF]
}

/// 槽 posX 读（F186 `$30E+Y` 面）。
pub fn read_pos_x(ram: &[u8], y: u8) -> u8 {
    ram[(OBJECT_BASE + OFF_POS_X + y as u16) as usize & 0x7FF]
}

/// 槽 hi 原始读（F186 `$30C+Y` 面；EOR zp $FF 变换在适配层——F186 专属协议）。
pub fn read_hi_raw(ram: &[u8], y: u8) -> u8 {
    ram[(OBJECT_BASE + OFF_HI + y as u16) as usize & 0x7FF]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 触达子集读（地址只出本层常量）。
    #[test]
    fn object_subset_reads() {
        let mut ram = vec![0u8; 0x800];
        ram[0x0301] = 8;
        ram[0x0302] = 12;
        ram[0x030C] = 0x01;
        ram[0x030D] = 0x50;
        ram[0x030E] = 0x60;
        assert_eq!(read_radius_y(&ram), 8);
        assert_eq!(read_radius_x(&ram), 12);
        assert_eq!(read_pos_y(&ram, 0), 0x50);
        assert_eq!(read_pos_x(&ram, 0), 0x60);
        assert_eq!(read_hi_raw(&ram, 0), 0x01);
    }
}
