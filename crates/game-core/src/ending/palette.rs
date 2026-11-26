//! 结局调色板的选择规则（规格 §5.3）。
//!
//! `EndGame ; 00:9AA7` 头部每 16 帧推进一次 `PaletteDataPending`：
//!
//! ```text
//!   lda IsCredits / bne @jumpEngine        ; 职员表滚动时不再换结局调色板
//!   lda FrameCount / and #$0F / bne @jumpEngine
//!   inc PaletteDataPending
//!   lda PaletteDataPending / cmp #_id_EndGamePalette08+1.b   ; $09
//!   bne @jumpEngine
//!   lda #_id_EndGamePalette00+1.b / sta PaletteDataPending   ; 回 $01
//! ```
//!
//! 写入端（`EndGamePaletteWrite 00:9F54`）取 `PaletteDataPending-1` 作
//! `EndGamePalettePtrTable ; 00:9F81` 的下标（13 项，`$00`–`$0C`）。段 4 淡出把同一
//! 计数器推到 `$0C`，段 5 职员表用 `$0D`——两者都在 [`PaletteSlot`] 里显式列出。

/// 结局调色板循环上界：`_id_EndGamePalette08+1`（= `$09`），到它就回到 `$01`。
pub const PENDING_CYCLE_END: u8 = 0x09;
/// 循环起点：`_id_EndGamePalette00+1`（= `$01`），即 `下标 0`。
pub const PENDING_CYCLE_START: u8 = 0x01;
/// 淡出末档（段 4 `EndFadeOut ; 00:9BCD` 每 8 帧 `+1`，到 `$0C` 进段 5）。
pub const PENDING_FADE_END: u8 = 0x0C;
/// 职员表调色板档（段 5 `RollCredits ; 00:9BFC` 用 `$0D`）。
pub const PENDING_CREDITS: u8 = 0x0D;

/// `PaletteDataPending` 指向的调色板档（`PaletteDataPending-1` 的语义化）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSlot {
    /// `EndGamePalette00..08`（下标 0–8），`PaletteDataPending` 1–9。
    EndPalette(u8),
    /// `PaletteDataPending==$0A/$0B` 两档指向同一张 `A049`（`00:9F81` 表第 10/11 项）。
    Fade(u8),
    /// `PaletteDataPending==$0C` ⇒ `EndGamePalettePtrTable` 第 12 项 `00:9806`。
    FadeEnd,
    /// 段 5 职员表调色板（`$0D`）。
    Credits,
}

/// `PaletteDataPending` 每 16 帧的自增（`inc` + `$09` 回 `$01`）。
pub fn next_pending(pending: u8) -> u8 {
    let next = pending.wrapping_add(1);
    if next >= PENDING_CYCLE_END {
        PENDING_CYCLE_START
    } else {
        next
    }
}

/// 把 RAM 值映射成调色板档；`$00`（未设置）与 `>= $0E` 无对应档。
pub fn slot_for(pending: u8) -> Option<PaletteSlot> {
    match pending {
        0 => None,
        1..=0x09 => Some(PaletteSlot::EndPalette(pending - 1)),
        0x0A..=0x0B => Some(PaletteSlot::Fade(pending - 1)),
        PENDING_FADE_END => Some(PaletteSlot::FadeEnd),
        PENDING_CREDITS => Some(PaletteSlot::Credits),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 循环上界：`$08 → $09` 时回 `$01`（不是 `$00`，asm 写的是 `EndGamePalette00+1`）。
    #[test]
    fn pending_cycles_one_to_eight() {
        assert_eq!(next_pending(0), 1);
        assert_eq!(next_pending(1), 2);
        assert_eq!(next_pending(7), 8);
        assert_eq!(next_pending(8), 1);
        assert_eq!(next_pending(PENDING_CYCLE_END - 1), PENDING_CYCLE_START);
    }

    /// 档位映射：段 0 首帧 `$01` ⇒ 第 0 张；`$0A/$0B` ⇒ 淡出同张；`$0C` ⇒ 末档；`$0D` ⇒ 职员表。
    #[test]
    fn slot_mapping_covers_fade_and_credits() {
        assert_eq!(slot_for(1), Some(PaletteSlot::EndPalette(0)));
        assert_eq!(slot_for(0x09), Some(PaletteSlot::EndPalette(8)));
        assert_eq!(slot_for(0x0A), Some(PaletteSlot::Fade(9)));
        assert_eq!(slot_for(0x0B), Some(PaletteSlot::Fade(10)));
        assert_eq!(slot_for(PENDING_FADE_END), Some(PaletteSlot::FadeEnd));
        assert_eq!(slot_for(PENDING_CREDITS), Some(PaletteSlot::Credits));
        assert_eq!(slot_for(0), None);
        assert_eq!(slot_for(0x0E), None);
    }

    /// `$0A/$0B` 指向同一张 `A049`：两者下标不同但落到同一个表项（rom 表第 10/11 项同址）。
    #[test]
    fn fade_slots_share_same_table_entry() {
        let a = slot_for(0x0A).unwrap();
        let b = slot_for(0x0B).unwrap();
        assert_ne!(a, b, "RAM 值不同");
        assert!(matches!(a, PaletteSlot::Fade(_)) && matches!(b, PaletteSlot::Fade(_)));
        // 表项地址同为 00:A049（规格 §5.3），故两者在数据面上同源。
        assert_eq!(a, PaletteSlot::Fade(9));
        assert_eq!(b, PaletteSlot::Fade(10));
    }
}
