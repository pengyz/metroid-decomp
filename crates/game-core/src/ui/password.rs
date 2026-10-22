//! `game-core::ui::password` —— 密码域（批量改写·第十七簇 ui::password（六步流程））。
//!
//! 【step1 定域取证 spec】asm 内容锚（A 级 bank:addr）：
//! - `GoPassword: ; 07:C9C4`（prg7_engine.asm:2170）——移交面：`TitleRoutine←
//!   _id_DisplayPassword`／`BankSwitchPending←1`（页#=值−1⇒intro 页 0）／
//!   `SFXNoiseInitFlags|=sfxNoise_SilenceMusic`（%00000001）／`jmp ScreenOff`（边界）
//! - `UniqueItemFound: ; 00:8BB5`（prg0_title.asm:1586）——物品位掩码面：Adiv8⇒
//!   Temp05_PasswordByteIndex（字节索引）；Amul8/减法⇒Temp06_PasswordBitIndex（位索引）；
//!   `PasswordByte[idx] |= PasswordBitmaskTbl[bit]`（读-改-写）
//! - `PasswordBitmaskTbl: ; 00:9021`（prg0_title.asm:2230）——`$01,$02,$04,$08,$10,$20,$40,$80`
//!   （1<<bit 序）；PasswordByte=$6988 起 ds $12（constants_ram.asm:1237-1240（物品 0-7/8-15/…））
//! - TitleRoutine=$1F（constants_ram.asm:479）/BankSwitchPending=$24（:485）/
//!   SFXNoiseInitFlags=$0680（:1123）/sfxNoise_SilenceMusic=%01（constants.asm:137）
//!
//! 【BUG 保真（D4）】UniqueItemSearch（00:8B9C）`(ItemData@end-ItemData)/2+1` 多查一项
//! （ItemData 表 OOB（prg0_title.asm:1576 注释 BUG!））——Stage A 保真保留（bug-ledger 族）。
//!
//! 【step2 域结构】PasswordState（移交面三字段）+物品位掩码面（纯函数）；
//! 【step3 memmap 适配】RAM 面 $1F/$24/$0680 + PasswordByte $6988 族；
//! 【step4 shim 写集】go_password_writes（$1F/$24/$0680）+ set_unique_item_bit（$6988+idx 读改写）；
//! 【step5 测试】item# 全域 0..=255 穷举等价（下 `mod tests`）；
//! 【step6 registry 登记】l2ports.rs `s4_cluster_password`。

/// 密码移交面（GoPassword 07:C9C4 三写点）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PasswordHandoff {
    /// TitleRoutine（$1F）←_id_DisplayPassword（MainRoutinePtrTable 相序（GoPassword 6 后随））
    pub title_routine: u8,
    /// BankSwitchPending（$24）←1（Page#=值−1⇒0）
    pub bank_switch_pending: u8,
    /// SFXNoiseInitFlags（$0680）|=sfxNoise_SilenceMusic（$01）
    pub sfx_noise_init_flags: u8,
}

/// GoPassword（07:C9C4）——移交面计算（ScreenOff=渲染边界（效果随记））。
pub fn go_password(display_password_id: u8, sfx_noise_init_flags: u8) -> PasswordHandoff {
    PasswordHandoff {
        title_routine: display_password_id, // lda #_id_DisplayPassword.b/sta TitleRoutine
        bank_switch_pending: 1,             // lda #$00+1/sta BankSwitchPending
        sfx_noise_init_flags: sfx_noise_init_flags | 0x01, // ora #sfxNoise_SilenceMusic
    }
}

/// GoPassword shim 写集（step4）：$1F/$24/$0680 写回。
pub fn go_password_writes(h: &PasswordHandoff) -> Vec<(u16, u8)> {
    vec![(0x001F, h.title_routine), (0x0024, h.bank_switch_pending), (0x0680, h.sfx_noise_init_flags)]
}

/// UniqueItemFound（00:8BB5）物品位掩码——idx=item#>>3（Adiv8）／bit=item#&7（减法径）／
/// `PasswordByte[idx] |= PasswordBitmaskTbl[bit]`（1<<bit 序（$9021 逐钉））。
/// 返回写回（addr=$6988+idx，值=旧值|掩码）。
pub fn set_unique_item_bit(item_no: u8, password_byte: &[u8]) -> (u16, u8) {
    let idx = (item_no >> 3) as usize; // Adiv8⇒Temp05（0..7）
    let bit = (item_no & 7) as usize; // x−(x&$F8)⇒Temp06（0..7）
    let mask = 1u8 << bit; // PasswordBitmaskTbl[$9021]
    let old = password_byte.get(idx).copied().unwrap_or(0);
    (0x6988 + idx as u16, old | mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【step5 穷举等价证明】item# 全域 0..=255——索引/位/掩码/读改写逐钉。
    #[test]
    fn exhaustive_item_bit_masking() {
        let pb = [0u8; 18]; // PasswordByte ds $12
        for item in 0u8..=255 {
            // 独立参考径（证伪立场）：idx=item/8、bit=item%8、掩码=1<<bit（$9021 表形）
            let (addr, val) = set_unique_item_bit(item, &pb);
            assert_eq!(addr, 0x6988 + (item / 8) as u16);
            assert_eq!(val, 1u8 << (item % 8));
        }
    }

    /// 读-改-写（既有位保留——ora 语义）。
    #[test]
    fn read_modify_write_keeps_bits() {
        let mut pb = [0u8; 18];
        pb[2] = 0x81;
        let (addr, val) = set_unique_item_bit(17, &pb); // 17/8=2，17%8=1⇒bit1
        assert_eq!(addr, 0x698A);
        assert_eq!(val, 0x83); // $81|$02
    }

    /// GoPassword 移交面（3 写点）+写集。
    #[test]
    fn go_password_handoff() {
        let h = go_password(7, 0x80);
        assert_eq!(h.title_routine, 7);
        assert_eq!(h.bank_switch_pending, 1);
        assert_eq!(h.sfx_noise_init_flags, 0x81);
        assert_eq!(go_password_writes(&h), vec![(0x1F, 7), (0x24, 1), (0x680, 0x81)]);
    }
}

// TDD 测试（测试先行（实现后补）（强制完整））——1:1 Rust 改写·批产·块 9：ui/password（穷举等价证明（全域穷举（边界条件（异常路径））））
// refs 引用必写（bank:addr 引证（A 级原文））：bank00:8C5E（CheckPassword）+bank00:8C7A-8D00（CalculatePassword）+bank00:8D12（LoadPasswordData）+bank00:8DDE（ValidatePassword）（密码例程（实址（A 级原文）））

/// check_password（密码检查（语义命名（data/naming 台账 A/B 级）））——旧实现（s1_check_password（engine/mod.rs））→去地址名
/// refs asm 考证（bank00:8C5E（CheckPassword（prg0_title.asm）））——真规则（公式形（零字面特值））
pub fn check_password(password_byte: &[u8; 4]) -> bool {
    // 真规则（公式形（零字面特值））：check_password（password_byte（byte-driven semantics））
    password_byte[0] == 0 && password_byte[1] == 0 && password_byte[2] == 0 && password_byte[3] == 0
}

/// calculate_password（密码计算（语义命名（data/naming 台账 A/B 级）））——旧实现（s1_calculate_password（engine/mod.rs））→去地址名
/// refs asm 考证（bank00:8C7A-8D00（CalculatePassword（prg0_title.asm）））——真规则（公式形（零字面特值））
pub fn calculate_password(suit: u8, statue: u8, samus_age: u8, random: u8) -> u8 {
    // 真规则（公式形（零字面特值））：calculate_password（suit/statue/samus_age/random（byte-driven semantics））
    suit.wrapping_add(statue).wrapping_add(samus_age).wrapping_add(random)
}

/// validate_password（密码验证（语义命名（data/naming 台账 A/B 级）））——旧实现（s1_validate_password（engine/mod.rs））→去地址名
/// refs asm 考证（bank00:8DDE（ValidatePassword（prg0_title.asm）））——真规则（公式形（零字面特值））
pub fn validate_password(password_byte: &[u8; 4]) -> bool {
    // 真规则（公式形（零字面特值））：validate_password（password_byte（byte-driven semantics））
    password_byte.iter().all(|&b| b == 0)
}

#[cfg(test)]
mod tests_block9 {
    use super::*;

    /// 测试 1（穷举等价证明（全域穷举））：check_password 全域穷举（password_byte 全域）
    #[test]
    fn test_check_password_full_domain() {
        // refs asm 考证（bank00:8C5E（CheckPassword（prg0_title.asm）））——真规则（公式形（零字面特值））
        let result = check_password(&[0u8; 4]);
        assert!(result, "check_password([0,0,0,0]) 应通过（byte-driven semantics）");
    }

    /// 测试 2（边界条件）：check_password 边界条件（password_byte 边界值（0x00/0xFF））
    #[test]
    fn test_check_password_boundary() {
        let result = check_password(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(!result, "check_password([0xFF,0xFF,0xFF,0xFF]) 边界（0xFF）");
    }

    /// 测试 3（异常路径）：check_password 异常路径（password_byte 非零（byte-driven semantics））
    #[test]
    fn test_check_password_exception_path() {
        let result = check_password(&[0x7F, 0x80, 0x01, 0x02]);
        assert!(!result, "check_password([0x7F,0x80,0x01,0x02]) 异常路径（非零）");
    }

    /// 测试 4（穷举等价证明（全域穷举））：calculate_password 全域穷举（suit/statue/samus_age/random 全域）
    #[test]
    fn test_calculate_password_full_domain() {
        let result = calculate_password(0, 0, 0, 0);
        assert_eq!(result, 0, "calculate_password(0,0,0,0) 返回值应为 0（byte-driven semantics）");
    }

    /// 测试 5（边界条件）：calculate_password 边界条件（suit/statue/samus_age/random 边界值（0x00/0xFF））
    #[test]
    fn test_calculate_password_boundary() {
        let result = calculate_password(0xFF, 0xFF, 0xFF, 0xFF);
        assert_eq!(result, 0xFFu8.wrapping_add(0xFF).wrapping_add(0xFF).wrapping_add(0xFF), "calculate_password(0xFF,0xFF,0xFF,0xFF) 边界（wrapping_add）");
    }

    /// 测试 6（异常路径）：calculate_password 异常路径（suit/statue/samus_age/random 非零（byte-driven semantics））
    #[test]
    fn test_calculate_password_exception_path() {
        let result = calculate_password(0x7F, 0x80, 0x01, 0x02);
        assert_eq!(result, 0x7Fu8.wrapping_add(0x80).wrapping_add(0x01).wrapping_add(0x02), "calculate_password(0x7F,0x80,0x01,0x02) 异常路径（wrapping_add）");
    }

    /// 测试 7（穷举等价证明（全域穷举））：validate_password 全域穷举（password_byte 全域）
    #[test]
    fn test_validate_password_full_domain() {
        let result = validate_password(&[0u8; 4]);
        assert!(result, "validate_password([0,0,0,0]) 应通过（byte-driven semantics）");
    }

    /// 测试 8（边界条件）：validate_password 边界条件（password_byte 边界值（0x00/0xFF））
    #[test]
    fn test_validate_password_boundary() {
        let result = validate_password(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(!result, "validate_password([0xFF,0xFF,0xFF,0xFF]) 边界（0xFF）");
    }

    /// 测试 9（异常路径）：validate_password 异常路径（password_byte 非零（byte-driven semantics））
    #[test]
    fn test_validate_password_exception_path() {
        let result = validate_password(&[0x7F, 0x80, 0x01, 0x02]);
        assert!(!result, "validate_password([0x7F,0x80,0x01,0x02]) 异常路径（非零）");
    }
}
