//! intro 星星闪烁族（窗口件 bank0:$8AC7 族；two-layer 域模块）。
//!
//! **asm 内容锚引证主体**（链规 17/施工令②③）——refs/m1disasm/SRC/prg0_title.asm
//! `StarPaletteSwitch: ; 00:8AC7`（注「Change star palette every 16th frame」/「Is any
//! other PPU data waiting? If so, exit」）：
//! ①16 帧门：`lda FrameCount / and #$0F / bne @RTS`；②队列门：`lda VRAMStructBufferIndex
//! / beq @checkSuccess`；③@checkSuccess：`lda #<$3F19 / sta Temp00_PaletteDestination`…
//! `lda IntroStarOffset / and #$07 / asl / tay` → 表查 IntroStarPalettePtrTable →
//! `inc IntroStarOffset` → `jsr PrepPPUPaletteString ($C37E)`（$3F19 面）→
//! `lda #<$3F1D`… `iny / jsr AddYToPtr02 ($C2B3)` → `jmp PrepPPUPaletteString`（$3F1D 面）。
//! ROM 抽查佐证：PtrTable prg[0x0AFF..0x0B0F]（8 项=8B0F/8B19/8B23/8B2D/8B37/8B41/8B4B/
//! 8B55 自证）；表数据 IntroStarPalette0-7（00:8B0F 族，8×10B）。
//!
//! 【更正 2026-10-15】前注「清零环 Y 型填充使 $07A0=0xA0 拦队列门/消费端先于生产端」
//! **撤回**——Y 型系清零环端口误读（asm `tya ;A = 0` 环外一次性=零填充；BNE 目标错位
//! 一位）；decomp 写点级捕获（$07A0 真机无 0xA0 写，写值域 boot {0,6,12}）反推定谳
//! （P28-F1 升级为模型更正案）。**更正后**：$07A0=0 ⇒ 队列门放行，生产端本单元先行
//! =真实执行序（engine 测试钉死）。
//! 生产/消费对子：本族=VRAM 写串**生产端**（经 PrepPPUPaletteString 入 VRAMStructBuffer
//! ——$07A0 面 **RAM watch 可观测**）；消费端=CheckVRAMStructBufferWrite（C2CA，NMI 工作
//! 体）——C37E 单元落地时成对端口。

use super::memory::FrameMemory;
use crate::ram_map::{FRAME_COUNT, INTRO_STAR_OFFSET, VRAM_STRUCT_INDEX};

/// IntroStarPalettePtrTable 8 项（00:8AFF；ROM 抽查自证——窗口 bank0 烘焙面）。
pub const INTRO_STAR_PALETTE_PTRS: [u16; 8] =
    [0x8B0F, 0x8B19, 0x8B23, 0x8B2D, 0x8B37, 0x8B41, 0x8B4B, 0x8B55];

/// StarPaletteSwitch 单元（boot 场景：FrameCount=0/队列空 ⇒ 进 @checkSuccess）。
pub fn star_palette_switch(mem: &mut FrameMemory) {
    // ①16 帧门
    if mem.read_ram(FRAME_COUNT) & 0x0F != 0 {
        return;
    }
    // ②队列门（$07A0≠0=VRAMStructBuffer 忙）
    if mem.read_ram(VRAM_STRUCT_INDEX) != 0 {
        return;
    }
    // ③@checkSuccess：表游标 → Temp02_PaletteSource（$02/$03）+ inc IntroStarOffset
    let idx = (mem.read_ram(INTRO_STAR_OFFSET) & 0x07) as usize;
    let ptr = INTRO_STAR_PALETTE_PTRS[idx];
    mem.write_ram(0x0002, ptr as u8 & 0xFF); // Temp02_PaletteSource lo
    mem.write_ram(0x0003, (ptr >> 8) as u8); // Temp02_PaletteSource hi
    mem.write_ram(INTRO_STAR_OFFSET, mem.read_ram(INTRO_STAR_OFFSET).wrapping_add(1));
    // $3F19 面 PrepPPUPaletteString（缓冲写串件——下一单元）
    crate::unported!("bank07:C37E");
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

    /// 16 帧门（FrameCount&0xF≠0 ⇒ 静默）。
    #[test]
    fn star_palette_16frame_gate() {
        let mut m = FrameMemory::default();
        m.write_ram(FRAME_COUNT, 3);
        star_palette_switch(&mut m); // 不炸=门拦
        assert_eq!(m.read_ram(INTRO_STAR_OFFSET), 0);
    }

    /// 队列门（$07A0≠0 ⇒ 静默）+ boot 路径（门全放→表游标态+C37E 引爆）。
    #[test]
    fn star_palette_queue_gate_and_boot_path() {
        let mut m = FrameMemory::default();
        m.write_ram(VRAM_STRUCT_INDEX, 1);
        star_palette_switch(&mut m); // 队列忙拦
        assert_eq!(m.read_ram(INTRO_STAR_OFFSET), 0);
        // boot 路径：FrameCount=0/队列空（清队列门态）
        m.write_ram(VRAM_STRUCT_INDEX, 0);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| star_palette_switch(&mut m)));
        let msg = panic_message(&r.unwrap_err());
        assert!(msg.contains("unported @ bank07:C37E"), "引爆点失准: {msg}");
        assert_eq!(m.read_ram(0x0003), 0x8B); // PtrTable[0]=8B0F hi
        assert_eq!(m.read_ram(0x0002), 0x0F); // lo
        assert_eq!(m.read_ram(INTRO_STAR_OFFSET), 1); // inc
    }
}
