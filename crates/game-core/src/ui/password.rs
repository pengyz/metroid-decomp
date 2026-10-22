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
