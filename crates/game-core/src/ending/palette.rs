//! 结局调色板的选择规则（规格 §5.3）。
//!
//! 推进端有两处，都只动 `PaletteDataPending`：
//!
//! ```text
//! EndGame ; 00:9AA7 头部（星空循环）
//!   lda IsCredits / bne @jumpEngine        ; 职员表滚动时不再换结局调色板
//!   lda FrameCount / and #$0F / bne @jumpEngine
//!   inc PaletteDataPending
//!   lda PaletteDataPending / cmp #_id_EndGamePalette08+1.b   ; $09
//!   bne @jumpEngine
//!   lda #_id_EndGamePalette00+1.b / sta PaletteDataPending   ; 回 $01
//!
//! EndFadeOut ; 00:9BCD（淡出）
//!   lda FrameCount / and #$07 / bne …      ; 每 8 帧
//!   inc PaletteDataPending
//!   lda PaletteDataPending / cmp #_id_EndGamePalette0B+1.b   ; $0C ⇒ 淡出结束
//!   bne …                                  ; 达到后 Timer3←$10、inc RoomPtr
//! ```
//!
//! 写入端（`EndGamePaletteWrite ; 00:9F54`）取 `idx = PaletteDataPending-1`，从
//! `EndGamePalettePtrTable ; 00:9F81`（13 项，`idx $00..$0C`）取两字节指针：
//!
//! ```text
//!   lda PaletteDataPending / beq @RTS        ; P=0 不写
//!   cmp #_id_EndGamePalette0B+1.b   ; $0C
//!   beq @RTS                                 ; P=$0C 直接返回（淡出结束帧不换调色板）
//!   cmp #_id_EndGamePalette0B+1+1.b ; $0D
//!   bne @endIf_A
//!       ldy #$00 / sty PaletteDataPending    ; P=$0D：写之前先把 P 清 0
//!   @endIf_A:
//!   asl / tay                                ; y = 2*P
//!   lda EndGamePalettePtrTable-1,y           ; 表项 = idx P-1 的高字节
//!   ldx EndGamePalettePtrTable-2,y           ; 低字节
//! ```
//!
//! 13 项表逐项（`prg0_title.asm:4472-4485`）：
//!
//! | P | idx=P-1 | 表项 | 地址 |
//! |---|---|---|---|
//! | `$01`..`$09` | 0..8 | `EndGamePalette00..08` | `$9F9B`..`$A013` |
//! | `$0A` | 9 | `EndGamePalette09` | `$A02E` |
//! | `$0B` | 10 | `EndGamePalette0A` | `$A049` |
//! | `$0C` | — | （`cmp #$0C / beq @RTS`，不写） | — |
//! | `$0D` | 12 | `EndGamePalette0B` | `$9806` |
//!
//! 注意 `idx 10` 与 `idx 11` 在表里同址 `$A049`，但 `$0C` 走的是 `beq @RTS` 早退，
//! **不会**落到 idx 11——所以「同址两张」在运行时不成立，`$0C` 只是淡出结束的触发器。

/// 星空循环上界：`_id_EndGamePalette08+1`（= `$09`），到它就回到 `$01`。
pub const PENDING_CYCLE_END: u8 = 0x09;
/// 循环起点：`_id_EndGamePalette00+1`（= `$01`），即表项下标 0。
pub const PENDING_CYCLE_START: u8 = 0x01;
/// 淡出末档：`EndFadeOut` 每 8 帧 `+1`，到 `$0B` 是最后一张表，再进到 `$0C` 触发结束。
pub const PENDING_FADE_LAST: u8 = 0x0B;
/// 淡出结束触发器：`EndFadeOut` 命中 `$0C` 时 `Timer3←$10`、`inc RoomPtr`；写入口对 `$0C` 早退。
pub const PENDING_FADE_TRIGGER: u8 = 0x0C;
/// 职员表调色板档：`RollCredits` 用 `$0D`；写入口对 `$0D` 先把 `PaletteDataPending` 清 0 再写。
pub const PENDING_CREDITS: u8 = 0x0D;

/// `PaletteDataPending` 指向的调色板档（`PaletteDataPending-1` 的语义化）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSlot {
    /// `EndGamePalette00..09`（表项下标 0–9），`PaletteDataPending` `$01`–`$0A`。
    EndPalette(u8),
    /// 淡出最后一张：`PaletteDataPending==$0B` ⇒ 表项下标 10，`00:A049`。
    Fade(u8),
    /// `PaletteDataPending==$0C`：淡出结束触发器，写入口 `beq @RTS`，**不写调色板**。
    FadeTrigger,
    /// 职员表：`PaletteDataPending==$0D` ⇒ 表项下标 12，`00:9806`；写后计数器清 0。
    Credits,
}

impl PaletteSlot {
    /// 表项下标（`EndGamePalettePtrTable` 的项号）；[`PaletteSlot::FadeTrigger`] 无表项。
    pub fn table_index(self) -> Option<u8> {
        match self {
            PaletteSlot::EndPalette(index) | PaletteSlot::Fade(index) => Some(index),
            PaletteSlot::FadeTrigger => None,
            PaletteSlot::Credits => Some(12),
        }
    }
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

/// 把 RAM 值映射成调色板档；`$00`（未设置）与 `>=$0E` 无对应档。
pub fn slot_for(pending: u8) -> Option<PaletteSlot> {
    match pending {
        0 => None,
        PENDING_CYCLE_START..=0x0A => Some(PaletteSlot::EndPalette(pending - 1)),
        PENDING_FADE_LAST => Some(PaletteSlot::Fade(10)),
        PENDING_FADE_TRIGGER => Some(PaletteSlot::FadeTrigger),
        PENDING_CREDITS => Some(PaletteSlot::Credits),
        _ => None,
    }
}

/// 写入口对 `PaletteDataPending` 的回写：`$0D` 写完清 0，其余保持原值。
pub fn pending_after_write(pending: u8) -> u8 {
    if pending == PENDING_CREDITS {
        0
    } else {
        pending
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

    /// 档位映射（按 `00:9F81` 表与 `00:9F54` 的三个比较逐值核对）。
    #[test]
    fn slot_mapping_follows_table_index_minus_one() {
        assert_eq!(slot_for(1), Some(PaletteSlot::EndPalette(0)));
        assert_eq!(slot_for(0x08), Some(PaletteSlot::EndPalette(7)));
        assert_eq!(slot_for(0x09), Some(PaletteSlot::EndPalette(8)));
        // $0A ⇒ 下标 9 = EndGamePalette09（$A02E），不是淡出同址档。
        assert_eq!(slot_for(0x0A), Some(PaletteSlot::EndPalette(9)));
        // $0B ⇒ 下标 10 = EndGamePalette0A（$A049），淡出最后一张。
        assert_eq!(slot_for(0x0B), Some(PaletteSlot::Fade(10)));
        // $0C ⇒ 写入口 `beq @RTS`：淡出结束触发器，不写调色板。
        assert_eq!(slot_for(0x0C), Some(PaletteSlot::FadeTrigger));
        // $0D ⇒ 下标 12 = EndGamePalette0B（$9806）。
        assert_eq!(slot_for(0x0D), Some(PaletteSlot::Credits));
        assert_eq!(slot_for(0), None);
        assert_eq!(slot_for(0x0E), None);
    }

    /// 表项下标：`$0A`=9、`$0B`=10、`$0D`=12；触发器无表项；同址只发生在表项 10/11，运行时到不了 11。
    #[test]
    fn table_index_matches_pointer_table() {
        assert_eq!(slot_for(0x0A).unwrap().table_index(), Some(9));
        assert_eq!(slot_for(0x0B).unwrap().table_index(), Some(10));
        assert_eq!(slot_for(0x0C).unwrap().table_index(), None);
        assert_eq!(slot_for(0x0D).unwrap().table_index(), Some(12));
        // 表项 10 与 11 同址 $A049，但 $0C 早退，故没有「P 落到 11」的路径。
        assert_eq!(slot_for(0x0B).unwrap().table_index(), Some(10));
    }

    /// `$0D` 写后计数器清 0（`ldy #$00 / sty PaletteDataPending`），其余不变。
    #[test]
    fn credits_write_resets_pending() {
        assert_eq!(pending_after_write(PENDING_CREDITS), 0);
        assert_eq!(pending_after_write(1), 1);
        assert_eq!(
            pending_after_write(PENDING_FADE_TRIGGER),
            PENDING_FADE_TRIGGER
        );
    }
}
