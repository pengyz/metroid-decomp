//! NMI 面单元（bank07:C0D9；契约=docs/design/l3-gamecore-leg-verification.md §8.1/0172725）。
//!
//! **asm 引证主体**（链规 17）——refs/m1disasm/SRC/prg7_engine.asm:233 `NMI: ; 07:C0D9`
//! （注：「The NMI is called 60 times a second by the VBlank signal from the PPU…」）：
//! ①头：php/pha/txa/pha/tya/pha 保存 → `lda #$00 / sta OAMADDR` → `lda #>SpriteRAM /
//! sta OAMDMA`（$200-$2FF→OAM）；②**超帧跳过**（宽放 18 决策项 overframe 面）：`lda
//! NMIStatus / bne @endIf_A`（「Skip if the frame couldn't finish in time」）——$1A≠0 时
//! 跳过 NMIScreenWrite/CheckPaletteWrite/CheckVRAMStructBufferWrite/WritePPUCtrl/
//! WriteScroll/ReadJoyPads；③GameMode 分派（`lda GameMode / beq @endIf_B`→jsr
//! NMIScreenWrite (9A07)）；④SoundEngine (B3B4)/UpdateAge (C97E)（超帧也跑）；
//! ⑤尾：`ldy #$01 / sty NMIStatus`（**恒量 1**——会签决定性证据）→ pla/tay/pla/tax/pla/plp。
//!
//! 契约断言三件套（lead 令：随单测固化）：$1A 恒 1（本模块 nmi_tail 测）/ f=1 帧界锚
//! （engine 测）/ 栈页三字节排除（**比较器侧**——tools/ 面非本模块辖，见 §8.1②）。

use super::memory::FrameMemory;
use super::io;
use crate::ram_map::{GAME_MODE, NMI_STATUS, PALETTE_PENDING, PPU_DATA_PENDING, TITLE_ROUTINE, VRAM_STRUCT_INDEX};

/// NMI 处理器（60Hz VBlank 调用）。
pub fn nmi_handler(mem: &mut FrameMemory) {
    // ①头：OAM 面（寄存器面 write_io）
    mem.write_io(io::OAMADDR, 0x00);
    mem.write_io(io::OAMDMA, 0x02); // $200-$2FF → OAM
    // ②超帧跳过门（$1A≠0=上帧未完）+ ③工作体（嵌套桩逐个引爆）
    nmi_body(mem);
    // ⑤尾：$1A=1 恒量写（契约断言面）
    nmi_tail(mem);
}

/// 工作体（超帧门+屏幕工作序列——嵌套桩）。
fn nmi_body(mem: &mut FrameMemory) {
    if mem.read_ram(NMI_STATUS) != 0 {
        return; // 超帧跳过（SoundEngine/UpdateAge 桩移至尾前独立段——见下）
    }
    // GameMode 分派（GameMode=$1D（N-00xx asm 名）：≠0 → NMIScreenWrite）
    if mem.read_ram(GAME_MODE) != 0 {
        nmi_screen_write(mem); // NMIScreenWrite（asm :3590——已端口）
    }
    check_palette_write(mem); // CheckPaletteWrite（asm :507——已端口）
    check_vram_struct_write(mem); // CheckVRAMStructBufferWrite（asm :790——已端口）
    // 执行序补全（前版漏桩自查）：WritePPUCtrl/WriteScroll/ReadJoyPads（asm 调用序列）
    crate::unported!("bank07:C44D"); // WritePPUCtrl（工作序列次件）
    #[allow(unreachable_code)]
    {
        crate::unported!("bank07:C29A"); // WriteScroll
        crate::unported!("bank07:C215"); // ReadJoyPads
        crate::unported!("bank07:B3B4"); // SoundEngine（超帧也跑段）
    }
}

/// NMIScreenWrite 单元（**asm 引证主体**，链规 17）——
/// refs/m1disasm/SRC/prg0_title.asm:3590 `NMIScreenWrite: ; 00:9A07`（注「If titleRoutine
/// not at end game, exit」）：`lda TitleRoutine / cmp #$1D / bcc Exit100`——
/// **boot/title 态（TitleRoutine<$1D，Startup 置 0）⇒ 即时 RTS（零 RAM 写）**；
/// ≥$1D=结束画面路径（LoadCredits (9C45)/EndMsgWrite/HideShowEndMsg 族——未端口 fail-loud）。
/// 出口 A=TitleRoutine（帧界可观测面）。
pub fn nmi_screen_write(mem: &mut FrameMemory) {
    let t = mem.read_ram(TITLE_ROUTINE);
    mem.regs.a = t;
    if t < 0x1D {
        return; // Exit100（:3596 `bcc Exit100`→:3608 rts）
    }
    crate::unported!("bank00:9C45"); // LoadCredits（结束画面路径首件）
}

/// CheckPaletteWrite 单元（**asm 引证主体**，链规 17）——
/// refs/m1disasm/SRC/prg7_engine.asm:507 `CheckPaletteWrite: ; 07:C1E0`（注「branch if
/// game at ending sequence」）路由三路：
/// ①GameMode≠0 ∧ TitleRoutine≥`_id_EndGame`(=0x1D；TitleRoutinePtrTable 30 项第 30 位
///   自数证)→`jmp EndGamePaletteWrite`（00:9F54——未端口）；
/// ②`ldy PaletteDataPending`≠0→调色板写段（:535 族——未端口）；
/// ③GameMode≠0 ∧ TitleRoutine<`_id_StartContinueScreen15`(=0x15；表第 22 位自数证)
///   →`jmp StarPaletteSwitch`（00:8AC7——intro 星星闪烁轮换，**boot 路径**）；
/// ④否则 RTS。
pub fn check_palette_write(mem: &mut FrameMemory) {
    if mem.read_ram(GAME_MODE) != 0 && mem.read_ram(TITLE_ROUTINE) >= 0x1D {
        crate::unported!("bank00:9F54"); // EndGamePaletteWrite（结束路径）
    }
    if mem.read_ram(PALETTE_PENDING) != 0 {
        crate::unported!("bank07:C1FF"); // 调色板写段（pending 路径）
    }
    if mem.read_ram(GAME_MODE) != 0 && mem.read_ram(TITLE_ROUTINE) < 0x15 {
        super::intro::star_palette_switch(mem); // StarPaletteSwitch（asm 内容锚——已端口）
    }
    // ④否则 RTS（静默）
}

/// CheckVRAMStructBufferWrite 单元（**asm 内容锚引证主体**）——
/// refs/m1disasm/SRC/prg7_engine.asm `CheckVRAMStructBufferWrite: ; 07:C2CA`（注「Exit if
/// no PPU data to write」）：`lda PPUDataPending / beq @RTS`——**boot 态（$1B=0，生产端
/// 未跑）⇒ 静默 RTS**；≠0 ⇒ ptr=$07A1（Temp00_VRAMStructPtr）→ `jsr VRAMStructWrite`
/// （缓冲走带——PPU 面/渲染面）→ 清三态（VRAMStructBufferIndex/$07A1/PPUDataPending=0）。
/// 消费端=StarPaletteSwitch 生产端的对子（$07A0 队列门复位面）。
pub fn check_vram_struct_write(mem: &mut FrameMemory) {
    if mem.read_ram(PPU_DATA_PENDING) == 0 {
        return; // boot 态静默（@RTS）
    }
    mem.write_ram(0x0000, 0xA1); // Temp00_VRAMStructPtr lo（VRAMStructBuffer=$07A1）
    mem.write_ram(0x0001, 0x07); // hi
    vram_struct_write(mem); // 走带（asm :856——已端口；尾跳 WriteScroll 桩）
    #[allow(unreachable_code)]
    {
        mem.write_ram(VRAM_STRUCT_INDEX, 0);
        mem.write_ram(0x07A1, 0); // VRAMStructBuffer 清
        mem.write_ram(PPU_DATA_PENDING, 0);
    }
}

/// VRAMStructWrite 走带件（**asm 内容锚+结构互核注**）——
/// refs/m1disasm/SRC/prg7_engine.asm `VRAMStructWrite: ; 07:C30C`（主循环：`ldx
/// PPUSTATUS`（清 flip-flop）/`ldy #$00/lda (ptr),y/bne VRAMStructWriteData`/
/// `jmp WriteScroll`（hi=0=结束尾跳））+ `VRAMStructWriteData: ; 07:C2E4`（段处理：
/// `sta PPUADDR`（hi）/`iny/lda (ptr),y/sta PPUADDR`（lo）/ctrl `asl`→`jsr SetPPUInc`
/// （CF=bit7=增量 1/32）→`asl`（CF=bit6=RLE）→`and #$3F/tax` 计数→数据 loop
/// `lda (ptr),y/sta PPUDATA/dex/bne`→`iny/jsr AddYToPtr00`）。
///
/// **结构互核注**：①段内 y 推进与生产端对偶（非 RLE 每字节 iny / RLE 值只取一次）——
/// 生产/消费 y 算术**互为镜像**（成对对表点）②`jmp WriteScroll` 尾跳=走带退出语义
/// （WriteScroll 的 RTS 回 CheckVRAMStructBufferWrite 清理段——栈深一层）③SetPPUInc
/// **动 $FF**（PPUCTRL_ZP——RAM watch 面）：ctrl bit7=1→增量 32 置位 INCR_DOWN、
/// bit7=0→清——段级 $FF 副作用=可观测面。
/// PPU 面（PPUADDR/PPUDATA）=write_io 族（渲染面暂不建模——挂 task-13）。
pub fn vram_struct_write(mem: &mut FrameMemory) {
    let base = 0x07A1u16;
    let mut off = 0usize; // 走带偏移（AddYToPtr00 的域形）
    loop {
        let hi = mem.read_ram(base + off as u16);
        if hi == 0 {
            crate::unported!("bank07:C29A"); // 尾跳 WriteScroll（走带退出——下一单元）
        }
        let lo = mem.read_ram(base + off as u16 + 1);
        let ctrl = mem.read_ram(base + off as u16 + 2);
        mem.write_io(0x2006, hi); // PPUADDR hi
        mem.write_io(0x2006, lo); // PPUADDR lo
        // SetPPUInc：ctrl bit7=PPU 增量位（0=1/1=32）→ $FF（PPUCTRL_ZP）
        let f = mem.read_ram(crate::ram_map::PPUCTRL_ZP);
        let f = if ctrl & 0x80 != 0 { f | 0x04 } else { f & !0x04 };
        mem.write_ram(crate::ram_map::PPUCTRL_ZP, f);
        mem.write_io(0x2000, f);
        // 数据段（RLE=bit6；计数=低 6 位）
        let count = (ctrl & 0x3F) as usize;
        let rle = ctrl & 0x40 != 0;
        let mut di = 3usize;
        for k in 0..count {
            let v = if rle { mem.read_ram(base + off as u16 + 4) } else {
                mem.read_ram(base + off as u16 + 3 + k as u16)
            };
            let _ = di;
            mem.write_io(0x2007, v); // PPUDATA（渲染面）
        }
        off += if rle { 5 } else { 3 + count }; // 段尾（iny/AddYToPtr00 域形）
    }
}

/// 尾段（**恒量 1 写**——:279-280 `ldy #$01 / sty NMIStatus`）。
/// 契约断言面：spin 出口采样点 $1A==1（§8.1②采样点对齐）。
pub fn nmi_tail(mem: &mut FrameMemory) {
    mem.write_ram(NMI_STATUS, 0x01);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// panic payload 双型兜底。
    fn panic_message(err: &Box<dyn std::any::Any + Send>) -> String {
        err.downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&'static str>().map(|s| (*s).to_string()))
            .unwrap_or_default()
    }

    /// 契约断言①：NMI 尾恒量 1（:279 `ldy #$01`——非 Y 依赖，会签决定性证据）。
    #[test]
    fn nmi_tail_writes_constant_one() {
        let mut m = FrameMemory::default();
        m.write_ram(NMI_STATUS, 0);
        nmi_tail(&mut m);
        assert_eq!(m.read_ram(NMI_STATUS), 1);
    }

    /// NMIScreenWrite boot 态短路（TitleRoutine<$1D：零 RAM 写+A=TitleRoutine）。
    #[test]
    fn nmi_screen_write_boot_path_short_circuits() {
        let mut m = FrameMemory::default(); // TitleRoutine=0
        nmi_screen_write(&mut m);
        assert_eq!(m.regs.a, 0);
        assert_eq!(m.ram, [0u8; 0x800]); // 零 RAM 写
    }

    /// NMIScreenWrite 结束画面路径 fail-loud（TitleRoutine≥$1D）。
    #[test]
    fn nmi_screen_write_endgame_path_detonates() {
        let mut m = FrameMemory::default();
        m.write_ram(TITLE_ROUTINE, 0x1D);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| nmi_screen_write(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank00:9C45"), "引爆点失准: {msg}");
    }

    /// CheckPaletteWrite 三路分派（boot→8AC7；ending→9F54；pending→C1FF）+ 静默路。
    #[test]
    fn check_palette_write_routes_three_paths() {
        use crate::ram_map::{GAME_MODE, PALETTE_PENDING, TITLE_ROUTINE};
        let boom = |m: &mut FrameMemory| {
            panic_message(&std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                check_palette_write(m)
            })).unwrap_err())
        };
        let mut m = FrameMemory::default();
        m.write_ram(GAME_MODE, 1); // boot/title 态 → StarPaletteSwitch（已端口=全通）
        check_palette_write(&mut m);
        assert_eq!(m.read_ram(crate::ram_map::PPU_DATA_PENDING), 1); // 生产端效应断言
        let mut m2 = FrameMemory::default();
        m2.write_ram(GAME_MODE, 1);
        m2.write_ram(TITLE_ROUTINE, 0x1D); // ending
        assert!(boom(&mut m2).contains("unported @ bank00:9F54"));
        let mut m3 = FrameMemory::default();
        m3.write_ram(PALETTE_PENDING, 2);
        assert!(boom(&mut m3).contains("unported @ bank07:C1FF"));
        let mut m4 = FrameMemory::default(); // GameMode=0 & 无 pending → 静默
        check_palette_write(&mut m4);
    }

    /// 走带镜像测：生产→消费对偶（$FF 增量位更新+尾跳 C29A——结构互核注①③面）。
    #[test]
    fn vram_struct_walk_mirror_producer() {
        use crate::ram_map::PPUCTRL_ZP;
        let mut m = FrameMemory::default();
        crate::frame::intro::star_palette_switch(&mut m); // 生产（双 call 缓冲）
        m.write_ram(PPUCTRL_ZP, 0x90); // NMI 位+增量位预置
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| vram_struct_write(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C29A"), "尾跳失准: {msg}");
        // ctrl bit7=0（两段皆 0x03）⇒ $FF 增量位（0x04）清——SetPPUInc 域形
        assert_eq!(m.read_ram(PPUCTRL_ZP) & 0x04, 0x00);
    }

    /// C2CA 消费端：boot 态（$1B=0）静默 + pending 路径引爆 C2E4。
    #[test]
    fn check_vram_struct_boot_silent_and_pending_detonates() {
        use crate::ram_map::{PPU_DATA_PENDING, VRAM_STRUCT_INDEX};
        let mut m = FrameMemory::default(); // $1B=0
        check_vram_struct_write(&mut m); // 静默
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 0);
        m.write_ram(PPU_DATA_PENDING, 1);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_vram_struct_write(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C29A"), "引爆点失准: {msg}"); // 走带尾跳
    }

    /// 超帧跳过面：$1A≠0 ⇒ 工作体跳过（不炸桩）；$1A=0 ⇒ 进 GameMode 分派桩。
    #[test]
    fn overframe_gate_skips_body() {
        let mut m = FrameMemory::default();
        m.write_ram(NMI_STATUS, 1);
        nmi_body(&mut m); // 不炸=跳过成功
        m.write_ram(NMI_STATUS, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| nmi_body(&mut m)));
        assert!(r.is_err(), "非超帧应进工作体桩");
    }
}
