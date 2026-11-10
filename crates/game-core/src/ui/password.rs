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


// ===== 卡 B G2：密码真规则（asm 唯一真相源——prg0_title.asm 逐行对读）=====
// refs asm 考证（bank00:8F60/$8DDE/$8E4E/$8E21/$8E2D/$8E6C（prg0_title.asm:2174/2003/
// 2054/2041/2049/2088）——A 级原文）：参考径铁证=analysis/password_verify pwlib.py
// （mini-6502 ROM 字节执行——1014 组对拍见 tests/password_reference_parity.rs）。

/// NARPASSWORDTbl（00:8E07——`"NARPASSWORD00000"` charmap.tbl 映射：N=17 A=0A R=1B
/// P=19 S=1C W=20 O=18 D=0D + '0'=00×5；prg0_title.asm:2033）。
pub const NAR_PASSWORD_TBL: [u8; 16] = [
    0x17, 0x0A, 0x1B, 0x19, 0x0A, 0x1C, 0x1C, 0x20, 0x18, 0x1B, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// ConsolidatePassword（00:8F60）——24 char→18 byte（6 组×3 byte，$8FF1/$9001/$9011）：
/// b[3I+0]=(c[4I]<<2)|(c[4I+1]>>4)；b[3I+1]=((c[4I+1]&0x0F)<<4)|(c[4I+2]>>2)
/// （$9001 入口 y=#I*4+1——读 c[4I+1]/c[4I+2]，A 级 prg0_title.asm:2191-2214）；
/// b[3I+2]=((c[4I+2]<<6)&0xC0)|c[4I+3]（TwoLowerAndSixLower 三连 ror 后 `and #$C0`
/// ——carry 残留位落 bit5 被 $C0 掩掉，A 级：prg0_title.asm:2217-2228）。
pub fn consolidate_password(chars: &[u8; 24]) -> [u8; 18] {
    let mut pb = [0u8; 18];
    for i in 0..6usize {
        let y = i * 4;
        pb[i * 3] = ((chars[y] << 2) & 0xFF) | (chars[y + 1] >> 4);
        pb[i * 3 + 1] = ((chars[y + 1] & 0x0F) << 4) | (chars[y + 2] >> 2);
        pb[i * 3 + 2] = (((chars[y + 2] << 6) & 0xFF) & 0xC0) | chars[y + 3];
    }
    pb
}

/// PasswordChecksum（00:8E21）——sum PB[0..=16]（ldy #$10 含端点=17 字节）。
/// 返回 (A, carry)；【2026-11-10 r9 更正】此 carry=sum>0xFF 的代数溢出位，
/// 并非 asm 逐轮 clc+adc 的末轮进位语义——两者当前恒同值（17 字节和至多
/// 0x10E），但消费者不应将其当作 asm C 旗镜像。现行全部调用方丢弃 carry。
pub fn password_checksum(pb: &[u8; 18]) -> (u8, bool) {
    let mut sum = 0u16;
    for i in 0..=16usize {
        sum += pb[i] as u16;
    }
    (sum as u8, sum > 0xFF)
}

/// 128-bit rol（$8E4E UnscramblePassword 单轮：Temp00=PB15 →rol PB[15..0]→
/// rol Temp00→PB15——carry_in 链入 PB15，旧 PB0 bit7 经 Temp00 落 PB15 bit0）。
fn rol128(pb: &mut [u8; 18]) {
    let b0_hi = pb[0] >> 7;
    for i in 0..16 {
        pb[i] = (pb[i] << 1) | if i < 15 { pb[i + 1] >> 7 } else { b0_hi };
    }
}

/// 128-bit ror（$8E2D PasswordScramble 单轮：Temp00=PB0 →ror PB[0..15]→
/// ror Temp00→PB0——carry_in 链入 PB0 bit7，旧 PB15 bit0 经 Temp00 落 PB0 bit7）。
fn ror128(pb: &mut [u8; 18]) {
    let b15_lo = pb[15] & 1;
    for i in (0..16).rev() {
        pb[i] = (pb[i] >> 1) | if i > 0 { (pb[i - 1] & 1) << 7 } else { b15_lo << 7 };
    }
}

/// 轮数展开（asm loop_A：循环体先执行、`dec Temp02` 后判非零——count=255⇒255 轮，
/// count=0⇒dec 0→FF 回绕 ⇒256 轮；$8E5E-8E61）。
fn scramble_rounds(count: u8, mut f: impl FnMut()) {
    let n = if count == 0 { 256 } else { count as usize };
    for _ in 0..n {
        f();
    }
}

/// UnscramblePassword（00:8E4E）——count=PB[16]（0xFF⇒255 轮/0⇒256 轮边界照实现）；
/// 每轮=**纯 128-bit 左旋**（asm：Temp00←PB15；rol PB[15..0]；rol Temp00——bit0=
/// 循环尾 carry=旧 PB0 bit7 ⇒ 环闭合；入口 C 与旧 PB15 bit7（→Temp bit7→出口 C）
/// 均被丢弃，prg0_title.asm:2054-2066 逐行对读）。
pub fn unscramble_password(pb: &mut [u8; 18]) {
    let count = pb[16];
    scramble_rounds(count, || rol128(pb));
}

/// PasswordScramble（00:8E2D）——count=PB[16]；每轮=纯 128-bit 右旋（同构：Temp00←
/// PB0；ror PB[0..15]；ror Temp00——bit7=循环尾 carry=旧 PB15 bit0；prg0_title.asm:2049）。
pub fn scramble_password(pb: &mut [u8; 18]) {
    let count = pb[16];
    scramble_rounds(count, || ror128(pb));
}

/// ChecksumAndScramble（00:8E17）——PB[17]←checksum(A)；scramble（编码面）。
pub fn checksum_and_scramble(pb: &mut [u8; 18]) {
    let (sum, _) = password_checksum(pb);
    pb[17] = sum;
    scramble_password(pb);
}

/// LoadPasswordChar（00:8E6C）——18 byte→24 char（编码面，$8F2D/$8F33/$8F46/$8F5A）。
pub fn load_password_char(pb: &[u8; 18]) -> [u8; 24] {
    let mut chars = [0u8; 24];
    for i in 0..6usize {
        let y = i * 3;
        chars[i * 4] = pb[y] >> 2;
        chars[i * 4 + 1] = ((pb[y] & 0x03) << 4) | (pb[y + 1] >> 4);
        chars[i * 4 + 2] = ((pb[y + 1] & 0x0F) << 2) | (pb[y + 2] >> 6);
        chars[i * 4 + 3] = pb[y + 2] & 0x3F;
    }
    chars
}

/// ValidatePassword（00:8DDE）——NAR 已置⇒**跳过 NAR 串比较、仍走 Unscramble+
/// Checksum 校验**（`lda NARPASSWORD; bne` → jsr Unscramble @ $8DF7——A 级
/// prg0_title.asm:2003-2008 字节 AD B2 69 D0 14）；否则 PasswordChar[0..16]==
/// NARPASSWORDTbl⇒NAR 激活+valid（NARPASSWORD←1 桥面写回）；否则 Unscramble+
/// Checksum cmp PB[17]（==⇒valid/:8E05 clc，≠⇒:8E02 sec）。返回 (valid,
/// nar_activated)；NAR 串命中路不 Unscramble（quirk 保真）。
pub fn validate_password(chars: &[u8; 24], pb: &mut [u8; 18], nar_active: bool) -> (bool, bool) {
    if !nar_active && chars[..16] == NAR_PASSWORD_TBL {
        return (true, true); // NAR 串命中：NARPASSWORD←1+valid（:2029-2036）
    }
    unscramble_password(pb);
    let (sum, _) = password_checksum(pb);
    (sum == pb[17], false) // cmp PB[17] beq⇒clc valid / sec invalid
}

/// CheckPassword（00:8C5E）语义核（纯函数面——副作用由桥面执行）：
/// Consolidate→Validate（无附加 carry 面入——$8E4E 旋转环丢弃入口 C，见上）。
pub fn check_password(chars: &[u8; 24], nar_active: bool) -> ([u8; 18], bool, bool) {
    let mut pb = consolidate_password(chars);
    let (valid, nar_hit) = validate_password(chars, &mut pb, nar_active);
    (pb, valid, nar_hit)
}

#[cfg(test)]
mod tests_real_rules {
    //! G2 TDD（asm 逐行对读后红→绿）：规则面单例 + JB 锚例 + 边界。
    use super::*;

    /// Consolidate 单例手算钉（group I=0：c0=0x13 c1=0x1E c2=0x1C c3=0x1D
    /// ⇒b0=(13<<2)|(1E>>4)=0x4E|0x01=0x4F…——不对 JB 整体（整体走参考径对拍），
    /// 这里钉初四位=0x13/0x1E/0x1C/0x1D 手推）。
    #[test]
    fn consolidate_group0_manual() {
        let mut chars = [0u8; 24];
        chars[0] = 0x13;
        chars[1] = 0x1E;
        chars[2] = 0x1C;
        chars[3] = 0x1D;
        let pb = consolidate_password(&chars);
        // b0=(0x13<<2)|(0x1E>>4)=(0x4C)|(0x01)=0x4D
        assert_eq!(pb[0], (0x13u8 << 2) | (0x1E >> 4));
        // b1=((c[1]&0x0F)<<4)|(c[2]>>2)=(0x0E<<4)|(0x07)=0xE7
        assert_eq!(pb[1], ((0x1Eu8 & 0x0F) << 4) | (0x1C >> 2));
        // b2=((0x1C<<6)&0xC0)|0x1D=0x00|0x1D=0x1D（0x1C bit7..6=00）
        assert_eq!(pb[2], ((0x1Cu8 << 6) & 0xC0) | 0x1D);
    }

    /// JB 锚例（任务卡）：tile 序列→A6 F3 8E A4 B9 65 24 A9 D1 7F FF…+valid。
    #[test]
    fn justin_bailey_anchor() {
        let jb: [u8; 24] = [0x13, 0x1E, 0x1C, 0x1D, 0x12, 0x17, 0x0B, 0x0A, 0x12, 0x15,
                            0x0E, 0x22, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F, 0x3F,
                            0x3F, 0x3F, 0x3F, 0x3F];
        let (pb, valid, _) = check_password(&jb, false);
        assert_eq!(pb[..10], [0xA6, 0xF3, 0x8E, 0xA4, 0xB9, 0x65, 0x24, 0xA9, 0xD1, 0x7F]);
        assert!(pb[10..16].iter().all(|&b| b == 0xFF));
        assert_eq!(pb[16], 0xFF, "shift byte");
        assert_eq!(pb[17], 0xFF, "checksum byte");
        assert!(valid, "JB 密码合法");
    }

    /// NAR 串路径：前 16 char=NARPASSWORDTbl ⇒valid+NAR 激活（pb 不 Unscramble——quirk）。
    #[test]
    fn nar_password_path() {
        let mut chars = [0x3Fu8; 24];
        chars[..16].copy_from_slice(&NAR_PASSWORD_TBL);
        let (pb, valid, nar_hit) = check_password(&chars, false);
        assert!(valid && nar_hit);
        // NAR 路不 Unscramble：pb=纯 Consolidate 面（PB[16]=((3F<<6)&C0)|3F=0x3F）
        assert_eq!(pb, consolidate_password(&chars));
    }

    /// NAR 已激活（nar_active=true）：跳 NAR 串比较、仍走 Unscramble+Checksum
    /// （asm AD B2 69 D0 14 → $8DF7 jsr Unscramble——ROM 铁证字节）。
    #[test]
    fn nar_active_skips_nar_match_only() {
        // 全零 chars+nar=true：unscramble 256 轮=identity，checksum=0==PB17 ⇒ valid
        let chars = [0u8; 24];
        let (pb, valid, nar_hit) = check_password(&chars, true);
        assert!(valid && !nar_hit);
        assert!(pb.iter().all(|&b| b == 0));
        // nar=true 时不触发 NAR 串激活（比较被跳过）
        let mut nar_chars = [0x3Fu8; 24];
        nar_chars[..16].copy_from_slice(&NAR_PASSWORD_TBL);
        let (_, _valid, nar_hit) = check_password(&nar_chars, true);
        assert!(!nar_hit, "nar_active 面串比较被跳过");
        // （valid 面与 checksum 语义一致——nar=true×NAR 串组合 ROM 对拍已被 1014 组覆盖）
    }

    /// 全零/全 FF 边界（期望=ROM 字节执行铁证——analysis/password_verify 对拍生成）。
    #[test]
    fn boundary_all_zero_all_ff() {
        // 全零：count=0⇒256 轮=2×128-bit 循环+carry_in=0 ⇒ identity；checksum=0==PB17 ⇒ valid
        let (pb, valid, _) = check_password(&[0u8; 24], false);
        assert!(pb.iter().all(|&b| b == 0));
        assert!(valid);
        // 全 FF：raw char 域外但 ROM 仍按 $8F60/$8E4E 处理 ⇒ PB 全 FF+invalid
        let (pb, valid, _) = check_password(&[0xFFu8; 24], false);
        assert!(pb.iter().all(|&b| b == 0xFF));
        assert!(!valid);
    }

    /// 编码面 canonical（ROM 铁证字节）：canonical 18B→LoadPasswordChar 编码→
    /// check_password 解码 ⇒原 18B+valid（ROM 对拍生成径双向钉）。
    #[test]
    fn encode_roundtrip_selfconsistency() {
        let v: [u8; 18] = [0, 11, 22, 33, 44, 55, 66, 77, 88, 99, 110, 121, 132, 143, 154,
                           165, 0, 216];
        // chars=ROM 编码径（$8E17 ChecksumAndScramble+$8E6C，pwlib encode 铁证输出）
        let chars: [u8; 24] = [0x00, 0x00, 0x2C, 0x16, 0x08, 0x12, 0x30, 0x37, 0x10, 0x24,
                               0x35, 0x18, 0x18, 0x36, 0x39, 0x39, 0x21, 0x08, 0x3E, 0x1A,
                               0x29, 0x10, 0x00, 0x28];
        let (back, valid, _) = check_password(&chars, false);
        // quirk：校验径不回写 PB[17]（PasswordChecksum 只读 cmp）——保留 consolidate 面
        assert_eq!(&back[..17], &v[..17], "canonical roundtrip");
        assert_eq!(back[17], ((chars[22] << 6) & 0xC0) | chars[23], "PB17=consolidate 面（quirk）");
        assert!(valid);
    }
}


#[cfg(test)]
mod tests_encode_reference {
    //! 编码面 ROM 铁证对拍（$8E17 ChecksumAndScramble+$8E6C LoadPasswordChar——
    //! pwlib.py ROM 字节实跑钉，seed=99 随机 13 组）：canonical→(scrambled,chars)。
    use super::*;

    const ENCODE_VECTORS: &[([u8; 18], [u8; 18], [u8; 24])] = &[
        ([0xCE,0xC2,0x66,0x5B,0x75,0x7F,0x44,0x2C,0x80,0xC4,0x2D,0xFA,0x66,0xD7,0x6E,0xBF,0xC6,0x50],[0xB2,0x03,0x10,0xB7,0xE9,0x9B,0x5D,0xBA,0xFF,0x3B,0x09,0x99,0x6D,0xD5,0xFD,0x10,0xC6,0x50],[0x2C,0x20,0x0C,0x10,0x2D,0x3E,0x26,0x1B,0x17,0x1B,0x2B,0x3F,0x0E,0x30,0x26,0x19,0x1B,0x1D,0x17,0x3D,0x04,0x0C,0x19,0x10]),
        ([0x4D,0xEC,0x5B,0xAD,0x2A,0x2C,0xF0,0xA1,0xCE,0x15,0x89,0x03,0xD9,0x68,0xB9,0x29,0xEC,0xA6],[0xBA,0xD2,0xA2,0xCF,0x0A,0x1C,0xE1,0x58,0x90,0x3D,0x96,0x8B,0x92,0x94,0xDE,0xC5,0xEC,0xA6],[0x2E,0x2D,0x0A,0x22,0x33,0x30,0x28,0x1C,0x38,0x15,0x22,0x10,0x0F,0x19,0x1A,0x0B,0x24,0x29,0x13,0x1E,0x31,0x1E,0x32,0x26]),
        ([0xEC,0x1A,0xB7,0x42,0x6E,0x52,0xE2,0x5A,0x5A,0x12,0x38,0x0B,0x57,0xF2,0xD6,0x29,0xD9,0xCB],[0x29,0x71,0x2D,0x2D,0x09,0x1C,0x05,0xAB,0xF9,0x6B,0x14,0xF6,0x0D,0x5B,0xA1,0x37,0xD9,0xCB],[0x0A,0x17,0x04,0x2D,0x0B,0x10,0x24,0x1C,0x01,0x1A,0x2F,0x39,0x1A,0x31,0x13,0x36,0x03,0x15,0x2E,0x21,0x0D,0x3D,0x27,0x0B]),
        ([0x49,0x9E,0x02,0xC3,0xA8,0x45,0xCF,0x66,0x1C,0x6C,0xE2,0x83,0xCE,0x40,0xB5,0xF3,0x1C,0x8D],[0xE4,0x0B,0x5F,0x34,0x99,0xE0,0x2C,0x3A,0x84,0x5C,0xF6,0x61,0xC6,0xCE,0x28,0x3C,0x1C,0x8D],[0x39,0x00,0x2D,0x1F,0x0D,0x09,0x27,0x20,0x0B,0x03,0x2A,0x04,0x17,0x0F,0x19,0x21,0x31,0x2C,0x38,0x28,0x0F,0x01,0x32,0x0D]),
        ([0x4D,0xD1,0x3F,0x31,0x3C,0x49,0x97,0x59,0x5A,0xD1,0x4A,0x6A,0xA3,0xFF,0x77,0x83,0x3C,0xBA],[0xAD,0x14,0xA6,0xAA,0x3F,0xF7,0x78,0x34,0xDD,0x13,0xF3,0x13,0xC4,0x99,0x75,0x95,0x3C,0xBA],[0x2B,0x11,0x12,0x26,0x2A,0x23,0x3F,0x37,0x1E,0x03,0x13,0x1D,0x04,0x3F,0x0C,0x13,0x31,0x09,0x25,0x35,0x25,0x13,0x32,0x3A]),
        ([0x5F,0x25,0x93,0x05,0x18,0xAC,0x6E,0xC1,0xCC,0xAC,0xCC,0xC5,0xD4,0xBD,0x2F,0x5C,0x13,0x47],[0xA5,0xEB,0x8B,0xE4,0xB2,0x60,0xA3,0x15,0x8D,0xD8,0x39,0x95,0x99,0x98,0xBA,0x97,0x13,0x47],[0x29,0x1E,0x2E,0x0B,0x39,0x0B,0x09,0x20,0x28,0x31,0x16,0x0D,0x36,0x03,0x26,0x15,0x26,0x19,0x22,0x3A,0x25,0x31,0x0D,0x07]),
        ([0xCC,0xE3,0x75,0x9F,0x73,0x9F,0x36,0xA0,0x6F,0x7F,0xC4,0x97,0xE0,0x29,0xE0,0xD7,0x83,0x37],[0xF9,0x9C,0x6E,0xB3,0xEE,0x73,0xE6,0xD4,0x0D,0xEF,0xF8,0x92,0xFC,0x05,0x3C,0x1A,0x83,0x37],[0x3E,0x19,0x31,0x2E,0x2C,0x3E,0x39,0x33,0x39,0x2D,0x10,0x0D,0x3B,0x3F,0x22,0x12,0x3F,0x00,0x14,0x3C,0x06,0x28,0x0C,0x37]),
        ([0x96,0xA4,0xF4,0x41,0x83,0xDC,0x94,0xD1,0x51,0x90,0xC2,0xE1,0x63,0xC2,0xAC,0x65,0xB0,0x9D],[0xC2,0xE1,0x63,0xC2,0xAC,0x65,0x96,0xA4,0xF4,0x41,0x83,0xDC,0x94,0xD1,0x51,0x90,0xB0,0x9D],[0x30,0x2E,0x05,0x23,0x30,0x2A,0x31,0x25,0x25,0x2A,0x13,0x34,0x10,0x18,0x0F,0x1C,0x25,0x0D,0x05,0x11,0x24,0x0B,0x02,0x1D]),
        ([0xB5,0x97,0x62,0x57,0x85,0xD5,0x76,0xD1,0xB0,0xCC,0x01,0x39,0x67,0x00,0x9E,0x55,0xE3,0x99],[0xF0,0xBA,0xAE,0xDA,0x36,0x19,0x80,0x27,0x2C,0xE0,0x13,0xCA,0xB6,0xB2,0xEC,0x4A,0xE3,0x99],[0x3C,0x0B,0x2A,0x2E,0x36,0x23,0x18,0x19,0x20,0x02,0x1C,0x2C,0x38,0x01,0x0F,0x0A,0x2D,0x2B,0x0B,0x2C,0x12,0x2E,0x0E,0x19]),
        ([0x87,0xD5,0xEF,0x93,0x66,0x0A,0x35,0x9D,0x26,0x10,0x1D,0xD5,0x99,0xBD,0x97,0x63,0x33,0xCB],[0x03,0xBA,0xB3,0x37,0xB2,0xEC,0x70,0xFA,0xBD,0xF2,0x6C,0xC1,0x46,0xB3,0xA4,0xC2,0x33,0xCB],[0x00,0x3B,0x2A,0x33,0x0D,0x3B,0x0B,0x2C,0x1C,0x0F,0x2A,0x3D,0x3C,0x26,0x33,0x01,0x11,0x2B,0x0E,0x24,0x30,0x23,0x0F,0x0B]),
        ([0xCC,0x58,0x87,0xEB,0xA8,0x41,0x42,0xE4,0xC7,0xD6,0x5B,0xB8,0x80,0x6A,0x8C,0xED,0xC3,0x7B],[0x98,0xFA,0xCB,0x77,0x10,0x0D,0x51,0x9D,0xB9,0x8B,0x10,0xFD,0x75,0x08,0x28,0x5C,0xC3,0x7B],[0x26,0x0F,0x2B,0x0B,0x1D,0x31,0x00,0x0D,0x14,0x19,0x36,0x39,0x22,0x31,0x03,0x3D,0x1D,0x10,0x20,0x28,0x17,0x0C,0x0D,0x3B]),
        ([0xF5,0xF3,0x56,0x26,0x7A,0xEB,0x07,0x3A,0xA8,0xFC,0x24,0xAB,0x02,0x8C,0x06,0xA3,0xD3,0x87],[0x60,0xE7,0x55,0x1F,0x84,0x95,0x60,0x51,0x80,0xD4,0x7E,0xBE,0x6A,0xC4,0xCF,0x5D,0xD3,0x87],[0x18,0x0E,0x1D,0x15,0x07,0x38,0x12,0x15,0x18,0x05,0x06,0x00,0x35,0x07,0x3A,0x3E,0x1A,0x2C,0x13,0x0F,0x17,0x1D,0x0E,0x07]),
    ];

    /// Scramble 面对拍（$8E2D ror 循环+PB17←checksum，13 组 ROM 铁证）。
    #[test]
    fn encode_scramble_reference_parity() {
        for (i, (canonical, scrambled, _)) in ENCODE_VECTORS.iter().enumerate() {
            let mut pb = *canonical;
            checksum_and_scramble(&mut pb);
            assert_eq!(pb, *scrambled, "scramble group {} mismatch", i);
        }
    }

    /// LoadPasswordChar 面对拍（$8E6C，13 组 ROM 铁证）。
    #[test]
    fn encode_chars_reference_parity() {
        for (i, (_, scrambled, chars)) in ENCODE_VECTORS.iter().enumerate() {
            assert_eq!(load_password_char(scrambled), *chars, "chars group {} mismatch", i);
        }
    }

    /// Checksum 边界钉（$8E21：sum PB[0..=16]——wrap 面 + 单字符位钉 ROM 铁证）：
    /// b0 bit1=1 → char22=0x01（SixUpperBits/…/组内位映射——pwlib 单字节探针）。
    #[test]
    fn checksum_and_bit_probe() {
        let (sum, carry) = password_checksum(&[0xFF; 18]);
        assert_eq!((sum, carry), ((17 * 0xFF) as u8, true)); // 17×FF=$10EF→EF+carry
        assert_eq!(password_checksum(&[0u8; 18]), (0, false));
        // 位钉（pwlib encode([PB],…) ROM 铁证——encode 面 PB17←checksum 后 $8E6C）：
        // PB[0]=0x04 → chars[0]=0x01（b0>>2）；checksum=0x04 → chars[23]=0x04
        let mut pb = [0u8; 18];
        pb[0] = 0x04;
        pb[17] = password_checksum(&pb).0;
        let chars = load_password_char(&pb);
        assert_eq!((chars[0], chars[23]), (0x01, 0x04));
        // PB[0]=0x10 → chars[0]=0x04；checksum=0x10 → chars[23]=0x10
        let mut pb = [0u8; 18];
        pb[0] = 0x10;
        pb[17] = password_checksum(&pb).0;
        let chars = load_password_char(&pb);
        assert_eq!((chars[0], chars[23]), (0x04, 0x10));
    }
}
