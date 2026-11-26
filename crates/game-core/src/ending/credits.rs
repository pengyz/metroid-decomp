//! `LoadCredits ; 00:9C45` 的索引公式（规格 §5.5）——职员表 44 条 VRAM 结构的选号。
//!
//! ```text
//! LoadCredits ; 00:9C45
//!   ldy CreditPageNumber
//!   beq @RTS                    ; page==0 不写
//!   cpy #$07 / bcs @RTS         ; page>=7 不写
//!   ldx #$00 / lda ScrollY
//!   bpl @endIf_A                ; bit7=0 ⇒ x=0，A=ScrollY
//!     inx / sec / sbc #$80      ; bit7=1 ⇒ x=1，A=ScrollY-$80
//! @endIf_A:
//!   cmp #$04 / bcs @RTS         ; (ScrollY&$7F)>=4 不写（露出的行还没到 4 行）
//!   sta $01
//!   dey                         ; page-1
//!   txa / bne @else_B
//!     dey / bmi @RTS            ; page<2 且 scroll<128 ⇒ 不写
//!     tya / asl asl asl / adc #$04     ; (page-2)*8+4
//!   @else_B:
//!     tya / asl asl asl                 ; (page-1)*8
//!   adc $01 / asl / tay
//!   ldx CreditsPtrTable,y       ; 索引 = (上文偏移 + (ScrollY&$7F)) * 2 字节
//! ```
//!
//! 记录号 = `scroll<128 ? (page-2)*8+4+sa : (page-1)*8+sa`，`sa = ScrollY&$7F ∈ 0..=3`；
//! 最大 `(6-1)*8+3 = 43`，正好是 `CreditsPtrTable` 的 44 项（`00:A291`–`00:A2E8`）。
//!
//! 段 5 的滚动/翻页/收尾（`RollCredits ; 00:9BFC`）现有实现仍在 `title/mod.rs:1211-1240`，
//! 改它属 `title/` 写面，本卡不复制一份；这里只提供可判定的纯函数。

/// `CreditsPtrTable` 条目数（`00:A291`–`00:A2E8` 共 44 项，索引 0–43）。
pub const CREDITS_ENTRIES: u8 = 44;
/// 最后一页（`CreditPageNumber`），到它滚到 `$88` 即 `inc RoomPtr` 收尾。
pub const LAST_PAGE: u8 = 6;
/// 段 5 收尾阈值（`CreditPageNumber==6 && ScrollY>=$88`）。
pub const SCROLL_FINISH: u8 = 0x88;
/// 段 5 换名表阈值（`ScrollY` 到 `$F0` ⇒ `PPUCTRL^=$02`、`ScrollY←0`、`Page+1`）。
pub const SCROLL_PAGE_TURN: u8 = 0xF0;
/// 露出的行数上限：`(ScrollY&$7F) >= 4` 时 `LoadCredits` 直接 RTS。
pub const SCROLL_WINDOW: u8 = 4;

/// `LoadCredits` 的记录号（0–43）；ROM 会 RTS 的组合返回 `None`。
///
/// 参数按 RAM 原值给：`page` = `CreditPageNumber($80)`，`scroll_y` = `ScrollY($FC)`。
pub fn record_index(page: u8, scroll_y: u8) -> Option<u8> {
    if page == 0 || page >= 0x07 {
        return None; // `beq @RTS` / `cpy #$07 bcs @RTS`
    }
    let sa = scroll_y & 0x7F;
    if sa >= SCROLL_WINDOW {
        return None; // `cmp #$04 bcs @RTS`
    }
    let idx = if scroll_y & 0x80 == 0 {
        // 名表 0（奇数页）：`(page-2)*8+4+sa`，page<2 时 ROM 走 `bmi @RTS`
        if page < 2 {
            return None;
        }
        (page - 2) * 8 + 4 + sa
    } else {
        // 名表 2（偶数页）：`(page-1)*8+sa`
        (page - 1) * 8 + sa
    };
    debug_assert!(idx < CREDITS_ENTRIES);
    Some(idx)
}

/// 段 5 的收尾判据：`CreditPageNumber==6 && ScrollY>=$88` ⇒ `inc RoomPtr`（进段 6）。
pub fn is_finished(page: u8, scroll_y: u8) -> bool {
    page == LAST_PAGE && scroll_y >= SCROLL_FINISH
}

/// 段 5 的一步滚动（`FrameCount&3==0` 时调用）：返回 `(新 ScrollY, 是否翻页)`。
///
/// 到 `$F0` ⇒ `PPUCTRL^=$02`、`ScrollY←0`、`CreditPageNumber+1`；翻页由调用方按
/// [`SCROLL_PAGE_TURN`] 处理（本函数只回报布尔）。
pub fn next_scroll(scroll_y: u8) -> (u8, bool) {
    if scroll_y >= SCROLL_PAGE_TURN {
        (0, true)
    } else {
        (scroll_y.wrapping_add(1), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 名表 0 分支：`(page-2)*8+4+sa`，并覆盖「page<2 不写」「sa>=4 不写」两个早退门。
    #[test]
    fn odd_page_rows_use_nametable0_formula() {
        assert_eq!(record_index(0, 0x00), None); // page==0
        assert_eq!(record_index(1, 0x00), None); // page<2 且 scroll<128 ⇒ bmi @RTS
        assert_eq!(record_index(1, 0x80), Some(0)); // scroll>=128：page1 合法
        assert_eq!(record_index(2, 0x00), Some(4)); // (2-2)*8+4+0
        assert_eq!(record_index(2, 0x03), Some(7)); // +sa=3
        assert_eq!(record_index(2, 0x04), None); // sa>=4 ⇒ cmp #$04 bcs @RTS
        assert_eq!(record_index(6, 0x03), Some(39)); // (6-2)*8+4+3
    }

    /// 名表 2 分支：`(page-1)*8+sa`，最大 43，正好落在 44 项表内。
    #[test]
    fn even_page_rows_use_nametable2_formula() {
        assert_eq!(record_index(2, 0x80), Some(8)); // (2-1)*8+0
        assert_eq!(record_index(3, 0x81), Some(17)); // (3-1)*8+1
        assert_eq!(record_index(6, 0x83), Some(43)); // 表尾
        assert_eq!(record_index(7, 0x80), None); // page>=7
        assert_eq!(record_index(6, 0x84), None); // sa 越窗
        assert!(record_index(6, 0x83).unwrap() < CREDITS_ENTRIES);
    }

    /// 段 5 收尾/翻页门。
    #[test]
    fn roll_finish_and_page_turn_thresholds() {
        assert!(!is_finished(6, 0x87));
        assert!(is_finished(6, 0x88));
        assert!(!is_finished(5, 0x88)); // 只有最后一页才收尾

        assert_eq!(next_scroll(0x00), (1, false));
        assert_eq!(next_scroll(0xEF), (0xF0, false));
        assert_eq!(next_scroll(0xF0), (0, true)); // 到 $F0 ⇒ 换名表、翻页
        assert_eq!(next_scroll(0xFF).0, 0); // 越界值按 >=$F0 处理
    }
}
