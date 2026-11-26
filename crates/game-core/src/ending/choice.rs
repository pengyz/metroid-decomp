//! `ChooseEnding ; 07:CAF5` 的 typed 视图（规格 §2）。
//!
//! 判定本身已在 `crate::title::choose_ending` 端口（含 D4 保真：asm 只查
//! `SamusAge+2`、不查 `+3`）。这里**不复制第二份判定**，只把它的两个裸 u8 结果
//! 包成 typed 值，并补上规格 §1.3 段 0/2 真正要用的派生量（`SpritePointerIndex`）。
//!
//! ```text
//! ChooseEnding ; 07:CAF5
//!   ldy #$01
//! @loop: lda SamusAge+2 / bne @endingFound
//!        lda SamusAge+1 / cmp AgeTable-1,y / bcs @endingFound
//!        iny / cpy #$05 / bne @loop
//! @endingFound: sty EndingType ; $6872
//!        lda #$00 / cpy #$04 / bcc @endIf_A / lda #$01
//! @endIf_A: sta JustInBailey      ; $69B3
//! ```
//!
//! `AgeTable ; 07:CB18 = 7A 16 0A 04`（`$7A`=37h 最差 … `$04`=最好-1）。

/// `AgeTable ; 07:CB18`（4 项，`+1` 字节阈值；`$7A` 37h / `$16` 6.7h / `$0A` 3.0h / `$04` 1.2h）。
pub const AGE_TABLE: [u8; 4] = [0x7A, 0x16, 0x0A, 0x04];

/// `EndingType ; $6872`（1=最差 … 5=最好）。
pub const ENDING_TYPE_MIN: u8 = 1;
/// 见上。
pub const ENDING_TYPE_MAX: u8 = 5;
/// `_id_EndingType04`：从这一档起 `JustInBailey=1`（无服）。
pub const SUITLESS_TYPE: u8 = 4;

/// `ChooseEnding` 的结果（`EndingType` + `JustInBailey`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndingChoice {
    /// `$6872` `EndingType`，1–5。
    pub ending_type: u8,
    /// `$69B3` `JustInBailey`：1=无服（`EndingType>=4`）。
    pub just_in_bailey: bool,
}

impl EndingChoice {
    /// 段 0 `SpritePointerIndex←EndingType*2`（`00:9AE5`：`asl`）。
    pub fn sprite_pointer_index(&self) -> u8 {
        self.ending_type.wrapping_mul(2)
    }

    /// `EndSamusAddrTbl ; 00:9D5A` 六帧时的下标（0=Normal … 5=Bikini）。
    ///
    /// `SpritePointerIndex = EndingType*2`，而表按 2 字节一项 ⇒ 表索引 = `EndingType`；
    /// 段 2 起 `SpritePointerIndex` 会被推进到下一帧，所以这里只表达「起始帧」。
    pub fn initial_samus_frame(&self) -> u8 {
        self.ending_type
    }
}

/// 按 `SamusAge`（4 字节小端：`$687D..$6880`）选结局，委托 `title::choose_ending`
///（唯一权威实现，`title/mod.rs:1941`）。
pub fn choose(samus_age: [u8; 4]) -> EndingChoice {
    let (ending_type, just_in_bailey) =
        crate::title::choose_ending([samus_age[0], samus_age[1], samus_age[2]]);
    EndingChoice {
        ending_type,
        just_in_bailey: just_in_bailey != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 阈值边界逐档命中（`AgeTable` 四项，`+1` 字节比较 `bcs`）。
    #[test]
    fn age_table_boundaries_select_ending() {
        assert_eq!(choose([0, 0x7A, 0, 0]).ending_type, 1);
        assert_eq!(choose([0, 0x16, 0, 0]).ending_type, 2);
        assert_eq!(choose([0, 0x0A, 0, 0]).ending_type, 3);
        assert_eq!(choose([0, 0x04, 0, 0]).ending_type, 4);
        // 比最低阈值还小 ⇒ 5（最好）
        assert_eq!(choose([0, 0x03, 0, 0]).ending_type, 5);
        // 边界下方一档要落到下一档（$16-1 仍是 2 档的 `>=` 判据不成立 ⇒ 3 档）
        assert_eq!(choose([0, 0x15, 0, 0]).ending_type, 3);
    }

    /// `SamusAge+2 != 0` ⇒ 直接最差（1 档），且不看 `+1`。
    #[test]
    fn age_plus_two_forces_worst_ending() {
        assert_eq!(choose([0, 0, 1, 0]).ending_type, 1);
        assert_eq!(choose([0, 0x7A, 1, 0]).ending_type, 1);
    }

    /// D4 保真：原版只查 `+2`，高字节溢出到 `+3` 不被检查（asm 注释自承 BUG）。
    #[test]
    fn rom_overflow_bug_is_preserved() {
        assert_eq!(choose([0, 0x03, 0, 1]).ending_type, 5);
        assert_eq!(choose([0, 0x03, 0, 1]).just_in_bailey, true);
    }

    /// 无服阈值：`EndingType>=4` ⇒ `JustInBailey=1`；3 档仍有服。
    #[test]
    fn suitless_threshold_and_sprite_index() {
        let e3 = choose([0, 0x0A, 0, 0]);
        assert_eq!((e3.ending_type, e3.just_in_bailey), (3, false));
        assert_eq!(e3.sprite_pointer_index(), 6);
        let e4 = choose([0, 0x04, 0, 0]);
        assert_eq!((e4.ending_type, e4.just_in_bailey), (4, true));
        assert_eq!(e4.sprite_pointer_index(), 8);
        assert_eq!(e4.initial_samus_frame(), 4);
    }
}
