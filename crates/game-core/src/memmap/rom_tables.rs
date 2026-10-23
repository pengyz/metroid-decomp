//! ROM 查询表烘焙面——**窗口参数化**（task-14，A9 证伪后遗处置）。
//!
//! 【2026-10-15 更正】旧「window_bank=7 约定」烘焙（l2-proof-assumptions.md A9）已被
//! decode-sweep r3 动态证伪（A 级窗口指纹 ×1600 帧：game 期 $8000 窗口=banks1-5 区域
//! bank，bank7 指纹 0 命中——docs/findings/decode-sweep-r3-dynamic-close.md §1.2）：
//! 窗口读表语义=**按区域查表**。烘焙面改 8 bank 全量 + `window_bank` 参数取数；
//! L2 契约 `window_bank` 字段=窗口输入面（缺省=entry bank 标签，既有案行为钉死）。
//!
//! 地址证据（A 级）：bank07:F29E `B9 8B 96` LDA $968B,Y（F282 流属性表查，Y<0x80）；
//! bank07:F309/F30E `85 6E`/`85 6F` STA zp $6E/$6F（F2ED 流向量搬运，源 $95CE/$95CF）。
//! 烘焙映射：窗口 CPU $8000+off → prg[window_bank × 0x4000 + off]。
//! 字节自证（一行复现）：
//! `python3 -c "p=open('rom/Metroid (USA).nes','rb').read()[0x10:0x20010]; print(p[w*0x4000+0x168B:w*0x4000+0x170B].hex())"`
//! 语义未证（C 级）——仅证 $968B 表 `AND #$10` 位测试（F2A1 `29 10`）与向量搬运；
//! 对拍面=b-f282-*-w*/b-f2ed-full-w*/b-f2e8-fall-full-w* 多窗口族（8 bank 全窗口三方全等）。

/// $968B 属性表逐 bank 烘焙（8×128B；Y∈[0,0x7F]——BMI 分流保证）。
/// 源：prg[w × 0x4000 + 0x168B .. w × 0x4000 + 0x170B]（逐 bank 自证见模块注）。
pub use game_data::rom_tables::TABLE_968B_BY_BANK;

/// 表读（**窗口参数化**；Y<0x80；F2A1 `AND #$10` 位测试面）。
/// `window_bank` 契约∈0..8（MMC1 16KB 窗口；越界=契约违规 fail-loud）。
pub fn table_968b(window_bank: usize, y: u8) -> u8 {
    TABLE_968B_BY_BANK[window_bank][(y & 0x7F) as usize]
}

/// $95CE/$95CF ROM 向量逐 bank 烘焙（8×2B，[lo, hi]）。
/// 源：prg[w × 0x4000 + 0x15CE .. w × 0x4000 + 0x15D0]。
pub use game_data::rom_tables::ROM_VEC_BY_BANK;

/// 向量读（**窗口参数化**）：(lo=$95CE, hi=$95CF)。契约同 [`table_968b`]。
pub fn rom_vec(window_bank: usize) -> (u8, u8) {
    let v = ROM_VEC_BY_BANK[window_bank];
    (v[0], v[1])
}

/// PPU nametable 高位表（bank07:C19F `20 24 28 2C`——C158 清屏目标 $2000/$2400/$2800/$2C00；
/// 索引=$01-1，A 级字节）。
pub use game_data::rom_tables::NAMETABLE_HI;

#[cfg(test)]
mod tests {
    use super::*;

    /// 窗口参数取数：同 Y 不同窗口命中不同 bank 切片（A9 处置核心行为）。
    #[test]
    fn table_968b_is_window_parameterized() {
        assert_eq!(table_968b(7, 0), 0x20); // 旧 window_bank=7 烘焙值（行为钉死）
        assert_eq!(table_968b(7, 1), 0x9A);
        assert_eq!(table_968b(0, 1), 0x1A); // 区域/标题 bank 与 bank7 不同字节
        assert_eq!(table_968b(3, 1), 0xFE);
        assert_eq!(table_968b(6, 1), 0x0E);
    }

    /// 向量逐 bank 锚点（ROM 字节自证值；bank7=旧常量 0x03/0x38 行为钉死）。
    #[test]
    fn rom_vec_per_bank_anchors() {
        assert_eq!(rom_vec(7), (0x03, 0x38)); // 旧 ROM_VEC_LO/HI（window_bank=7）
        assert_eq!(rom_vec(0), (0x3F, 0x00));
        assert_eq!(rom_vec(2), (0x00, 0x01));
        assert_eq!(rom_vec(5), (0x40, 0x02));
        assert_eq!(rom_vec(6), (0x25, 0x30));
    }

    /// Y 索引掩 0x7F（BMI 分流契约面）。
    #[test]
    fn table_968b_y_wraps_low7() {
        assert_eq!(table_968b(3, 0x80), table_968b(3, 0x00));
        assert_eq!(table_968b(3, 0xFF), table_968b(3, 0x7F));
    }
}
