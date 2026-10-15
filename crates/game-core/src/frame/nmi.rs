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
use crate::ram_map::{NMI_STATUS, TITLE_ROUTINE};

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
    if mem.read_ram(0x001D) != 0 {
        nmi_screen_write(mem); // NMIScreenWrite（asm :3590——已端口）
    }
    crate::unported!("bank07:C1E0"); // CheckPaletteWrite 起的屏幕工作序列
    #[allow(unreachable_code)]
    {
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
