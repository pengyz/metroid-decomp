//! game-core::ram_map —— 已自证 RAM 常量图（A 级地址 + B 级语义，证据指针随注）。
//!
//! 纪律（D5/AGENTS §3）：地址经 seed 字节自证=A 级；语义经 trace 行为相关性=B 级；
//! 第三方命名（m1disasm）仅作线索，本模块名字为自证后采用的候选名（台账登记待批量期统一）。

/// asm 命名采纳（链规 17/task-16 流：refs 候选→自证→台账 N-0014+）：

/// PPUMASK_ZP=$FE（refs/m1disasm/SRC/constants_ram.asm:797「Data byte to be loaded into
/// PPU control register 1」；ScreenNmiOff/WriteAndWait 面）。
pub const PPUMASK_ZP: u16 = 0x00FE;

/// PPUCTRL_ZP=$FF（constants_ram.asm:798「Data byte to be loaded into PPU control
/// register 0」；NMI 使能位镜像面）。
pub const PPUCTRL_ZP: u16 = 0x00FF;

/// NMIStatus=$1A（constants_ram.asm:474「0=NMI in progress. anything else, NMI not in
/// progress」）——**成环标志同源**（decomp e3f2473：$C0C9 清/NMI 端置，C0CC 轮询）。
pub const NMI_STATUS: u16 = 0x001A;

/// TitleRoutine=$1F（constants_ram.asm:479「Stores title routine number currently
/// running」；Startup `sty TitleRoutine`（Y=0）——boot/title 态 < $1D 短路面）。
pub const TITLE_ROUTINE: u16 = 0x001F;

/// GameMode=$1D（asm 名；InitBank0 `iny / sty GameMode`=1（title）自证；
/// prg7_engine NMI/CheckPaletteWrite 分派面）。
pub const GAME_MODE: u16 = 0x001D;

/// PaletteDataPending=$1C（constants_ram.asm:476「Pending palette data. Palette # =
/// PaletteDataPending - 1」）。
pub const PALETTE_PENDING: u16 = 0x001C;

/// FrameCount=$2D（constants_ram 内容锚「Increments every frame(overflows every 256
/// frames)」；主循环 `inc FrameCount`）。
pub const FRAME_COUNT: u16 = 0x002D;

/// IntroStarOffset=$C3（constants_ram 内容锚「Contains offset into IntroStarPntr table
/// for twinkle effect」）。
pub const INTRO_STAR_OFFSET: u16 = 0x00C3;

/// VRAMStructBufferIndex=$07A0（constants_ram 内容锚「# of bytes of data in
/// VRAMStructBuffer. #$4F bytes max」）——**RAM watch 面内**（iram $0100-$07FF）。
pub const VRAM_STRUCT_INDEX: u16 = 0x07A0;

/// PPUDataPending=$1B（constants_ram 内容锚「1=not PPU data pending, 1=data pending」
/// （社区注自相矛盾，取 PrepPPUPaletteString 写 1=待写语义））。
pub const PPU_DATA_PENDING: u16 = 0x001B;

/// ScrollX=$FD（constants_ram 内容锚「X value loaded into scroll register」；
/// boot 初始化 `sty $FD`（Y=0）自证）。
pub const SCROLL_X: u16 = 0x00FD;

/// ScrollY=$FC（constants_ram 内容锚「Y value loaded into scroll register」）。
pub const SCROLL_Y: u16 = 0x00FC;

/// MirrorCntrl=$FA（constants_ram 内容锚「If bit 3 is set, PPU set to horizontal
/// mirroring」；boot 初始化 `sta MirrorCntrl`（$47）自证）。
pub const MIRROR_CNTRL: u16 = 0x00FA;

/// MMC1CTRL_ZP=$25（constants_ram 内容锚「Stores bits to be loaded into MMC1 Register 0」）。
pub const MMC1CTRL_ZP: u16 = 0x0025;

/// Joy1Status=$14（constants_ram 内容锚「These two addresses store all buttons currently
/// being pressed on the two controllers」）。
pub const JOY1_STATUS: u16 = 0x0014;

/// Joy2Status=$15（同上注）。
pub const JOY2_STATUS: u16 = 0x0015;

/// Joy1Change=$12（constants_ram 内容锚「These addresses store any button changes」）。
pub const JOY1_CHANGE: u16 = 0x0012;

/// MainRoutine=$1E（constants_ram.asm:478「5 = Game paused, 3 = Game engine running」）。
pub const MAIN_ROUTINE: u16 = 0x001E;

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

/// 垂直速度整数部（有符号 16 位的高字节；与 SAMUS_SPEED_SUBPIXEL_Y 构成 frac+int）。
/// 地址证据（A 级）：seed bank07:E3B1 `AD 08 03` / E3B6 `8D 08 03`（metroid.asm:105395±）。
pub const SAMUS_SPEED_Y_INT: u16 = 0x0308;

/// 水平速度整数部（与 SAMUS_SPEED_SUBPIXEL_X 构成 frac+int）。
/// 地址证据（A 级）：seed bank07:E40D `6D 09 03` / E410 `8D 09 03`。
pub const SAMUS_SPEED_X_INT: u16 = 0x0309;

/// 垂直加速度增量输入。
/// 地址证据（A 级）：seed bank07:E3AB `6D 14 03` ADC $314。
pub const SAMUS_ACCEL_Y: u16 = 0x0314;

/// 水平加速度增量输入。
/// 地址证据（A 级）：seed bank07:E3FD `6D 15 03` ADC $315。
pub const SAMUS_ACCEL_X: u16 = 0x0315;

/// 水平最大速度来源输入（经 sub_E449 生成幅值钳位上限）。
/// 地址证据（A 级）：seed bank07:E3E5 `AD 16 03` LDA $316。
pub const SAMUS_MAXSPEED_X_SRC: u16 = 0x0316;

/// 垂直速度整数钳位常量（积分器语义，A 级指令证据）。
/// seed bank07:E3C3 `C9 06`（负向限幅 -6，LDX #$FA）/ E3C9 `C9 05`（正向限幅 5）。
pub const SAMUS_SPEED_Y_CLAMP_POS: i8 = 5;
pub const SAMUS_SPEED_Y_CLAMP_NEG: i8 = -6;

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
        assert_eq!(SAMUS_SPEED_Y_INT, 0x308);
        assert_eq!(SAMUS_SPEED_X_INT, 0x309);
        assert_eq!(SAMUS_ACCEL_Y, 0x314);
        assert_eq!(SAMUS_ACCEL_X, 0x315);
        assert_eq!(SAMUS_MAXSPEED_X_SRC, 0x316);
    }

    /// asm 命名采纳常量的地址自证锁（N-0014/15/16：constants_ram.asm 797/798/474 注址）。
    #[test]
    fn boot_face_constants_match_asm_addresses() {
        assert_eq!(PPUMASK_ZP, 0x00FE); // constants_ram.asm:797
        assert_eq!(PPUCTRL_ZP, 0x00FF); // constants_ram.asm:798
        assert_eq!(NMI_STATUS, 0x001A); // constants_ram.asm:474
    }
}
