//! game-core::ram_map —— 已自证 RAM 常量图（A 级地址 + B 级语义，证据指针随注）。
//!
//! 纪律（D5/AGENTS §3）：地址经 seed 字节自证=A 级；语义经 trace 行为相关性=B 级；
//! 第三方命名（m1disasm）仅作线索，本模块名字为自证后采用的候选名（台账登记待批量期统一）。

/// Samus 屏幕 X 坐标。
/// 语义证据（B 级）：samus-move3 trace——右行窗口 0x00→8E、左行 8F→70（单调随行向变化）；
/// 地址证据（A 级）：m1disasm constants_ram.asm 线索 $51 + seed 移动反应面（G4 findings A4）。
pub const SAMUS_SCR_X: u16 = 0x0051;

/// Samus 屏幕 Y 坐标。
/// 语义证据（B 级）：samus-move3 trace——过门后下落 B1→B9→C1 稳定（y 随下落增大）。
pub const SAMUS_SCR_Y: u16 = 0x0052;

/// 垂直亚像素累加器。
/// 地址证据（A 级）：seed bank07:E3DA `8D 10 03` STA $0310（metroid.asm:105413）。
pub const SAMUS_SUBPIXEL_Y: u16 = 0x0310;

/// 水平亚像素累加器。
/// 地址证据（A 级）：seed bank07:E43E `8D 11 03` STA $0311（metroid.asm:105458）。
/// 语义证据（B 级）：右行稳态 0x40 恒相位、停步后 0x80/50/70 游走（亚像素特征）。
pub const SAMUS_SUBPIXEL_X: u16 = 0x0311;

/// 垂直速度线性计数器。
/// 地址证据（A 级）：seed bank07:E2FD `8D 12 03` STA $0312（metroid.asm:105317）。
pub const SAMUS_SPEED_SUBPIXEL_Y: u16 = 0x0312;

/// 水平速度线性计数器。
/// 地址证据（A 级）：seed bank07:E400 `8D 13 03` STA $0313（metroid.asm:105429）。
/// 语义证据（B 级）：右行稳态恒 0x80（等速相位）、左行 0xD0（反向速度态）。
pub const SAMUS_SPEED_SUBPIXEL_X: u16 = 0x0313;

/// 跳跃起点垂直位移（补码像素计）。
/// 地址证据（A 级）：m1disasm 线索 $030F（Samus.data0F）+ seed 跳跃例程引用；
/// 语义证据（B 级）：trace FF→F7→EF 递增位移后稳定（落地）。
pub const SAMUS_JUMP_DISPLACEMENT: u16 = 0x030F;

/// UpdateSamus 例程入口（显示/移动总控）。
/// 地址证据（A 级）：seed bank07:CC0D `A2 00` LDX #$00（metroid.asm:101625），
/// 被 bank07:CB33 `JSR sub_CC0D`（metroid.asm:101522）调用。
pub const UPDATE_SAMUS_ENTRY: u16 = 0xCC0D;

/// 跳跃引擎分发表入口（ASL A 取 word 跳转）。
/// 地址证据（A 级）：seed bank07:C27C（metroid.asm:100448）；调用点 C13E/CC1F/CC7A。
pub const JUMP_ENGINE_ENTRY: u16 = 0xC27C;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_map_addresses_match_evidence() {
        // 防手滑回归：地址改动必须连带证据指针更新（本测试=最小锚）
        assert_eq!(SAMUS_SCR_X, 0x51);
        assert_eq!(SAMUS_SCR_Y, 0x52);
        assert_eq!(SAMUS_SUBPIXEL_Y, 0x310);
        assert_eq!(SAMUS_SUBPIXEL_X, 0x311);
        assert_eq!(SAMUS_SPEED_SUBPIXEL_Y, 0x312);
        assert_eq!(SAMUS_SPEED_SUBPIXEL_X, 0x313);
        assert_eq!(SAMUS_JUMP_DISPLACEMENT, 0x30F);
        assert_eq!(UPDATE_SAMUS_ENTRY, 0xCC0D);
        assert_eq!(JUMP_ENGINE_ENTRY, 0xC27C);
    }
}
