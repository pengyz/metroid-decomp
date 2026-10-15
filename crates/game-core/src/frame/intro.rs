//! intro 星星闪烁族（窗口件 bank0:$8AC7 族；two-layer 域模块）。
//!
//! **asm 内容锚引证主体**（施工令②精化版：锚证+**结构互核注**）——
//! refs/m1disasm/SRC/prg0_title.asm `StarPaletteSwitch: ; 00:8AC7`：
//! ①16 帧门（`lda FrameCount/and #$0F/bne @RTS`）②队列门（`lda VRAMStructBufferIndex/
//! beq @checkSuccess`）③表游标→ptr 装载→inc→call1（$3F19）→`iny`→AddYToPtr02→
//! call2（$3F1D）。
//!
//! **结构互核注**（施工令②补款——环界/分支目标/数据岛边界自证）：
//! · **y 复用三相**：表索引（`(offset&7)*2`）→ 解释器游标（call1 walk 终值=4）→
//!   `iny`=5 ⇒ ptr+5=**stream2 起点**（10B 块内两个 5B 流几何自证：`03|d d d|00`×2）；
//!   AddYToPtr02（`tya/clc/adc $02`）加的是 walk 残值非表索引——跨相复用易错点
//!   （与 TYA 案同族的相位误读面，以块几何 8×10B 双流反证）。
//! · **分支目标互核**：解释器数据节 `bne WritePaletteStringByte`（回数据节非段头）；
//!   `bvc/bvs` 双 iny 路径=非 RLE 每字节 iny、RLE 值字节只取一次（y 走 i+2）。
//! · **数据岛边界**：IntroStarPalette0-7（00:8B0F 族）8×10B——ROM 提取自证
//!   （prg[0x0B0F..0x0B5F]）。
//!
//! **生产/消费对子**：本族=VRAM 写串**生产端**（入 VRAMStructBuffer——$07A0 面 RAM
//! watch 可观测）；消费端=CheckVRAMStructBufferWrite（C2CA）。
//!
//! 【更正 2026-10-15】前注「清零环 Y 型填充使 $07A0=0xA0 拦队列门/消费端先于生产端」
//! **撤回**——Y 型系清零环端口误读（零填充更正，50af369）；更正后 $07A0=0 门放行，
//! 生产端先行=真实执行序。

use super::memory::FrameMemory;
use crate::ram_map::{
    FRAME_COUNT, INTRO_STAR_OFFSET, PPU_DATA_PENDING, VRAM_STRUCT_INDEX,
};

/// IntroStarPalettePtrTable 8 项（00:8AFF；ROM 抽查自证）。
pub const INTRO_STAR_PALETTE_PTRS: [u16; 8] =
    [0x8B0F, 0x8B19, 0x8B23, 0x8B2D, 0x8B37, 0x8B41, 0x8B4B, 0x8B55];

/// IntroStarPalette0-7 块（00:8B0F 族，8×10B——ROM 提取自证 prg[0x0B0F..0x0B5F]）。
/// 每块=两个 5B 流：`[ctrl=03][3B 色值][00]`。
pub const INTRO_STAR_PALETTES: [[u8; 10]; 8] = [
    [0x03, 0x0F, 0x02, 0x13, 0x00, 0x03, 0x00, 0x34, 0x0F, 0x00],
    [0x03, 0x06, 0x01, 0x23, 0x00, 0x03, 0x0F, 0x34, 0x09, 0x00],
    [0x03, 0x16, 0x0F, 0x23, 0x00, 0x03, 0x0F, 0x24, 0x1A, 0x00],
    [0x03, 0x17, 0x0F, 0x13, 0x00, 0x03, 0x00, 0x04, 0x28, 0x00],
    [0x03, 0x17, 0x01, 0x14, 0x00, 0x03, 0x10, 0x0F, 0x28, 0x00],
    [0x03, 0x16, 0x02, 0x0F, 0x00, 0x03, 0x30, 0x0F, 0x1A, 0x00],
    [0x03, 0x06, 0x12, 0x0F, 0x00, 0x03, 0x30, 0x04, 0x09, 0x00],
    [0x03, 0x0F, 0x12, 0x14, 0x00, 0x03, 0x10, 0x24, 0x0F, 0x00],
];

/// StarPaletteSwitch 单元（boot 场景：FrameCount=0/队列空 ⇒ 双 call 生产）。
pub fn star_palette_switch(mem: &mut FrameMemory) {
    // ①16 帧门
    if mem.read_ram(FRAME_COUNT) & 0x0F != 0 {
        return;
    }
    // ②队列门
    if mem.read_ram(VRAM_STRUCT_INDEX) != 0 {
        return;
    }
    // ③表游标 → Temp02_PaletteSource（$02/$03）+ inc
    let idx = (mem.read_ram(INTRO_STAR_OFFSET) & 0x07) as usize;
    let ptr = INTRO_STAR_PALETTE_PTRS[idx];
    mem.write_ram(0x0002, ptr as u8 & 0xFF);
    mem.write_ram(0x0003, (ptr >> 8) as u8);
    mem.write_ram(INTRO_STAR_OFFSET, mem.read_ram(INTRO_STAR_OFFSET).wrapping_add(1));
    let block = &INTRO_STAR_PALETTES[idx];
    // call1（$3F19 面）：stream1=block[0..5]
    mem.write_ram(0x0000, 0x19);
    mem.write_ram(0x0001, 0x3F);
    prep_ppu_palette_string(mem, &block[0..5]);
    // call2（$3F1D 面）：y walk 残值 4+iny=5 ⇒ ptr+5=stream2（结构互核注面）
    mem.write_ram(0x0000, 0x1D);
    mem.write_ram(0x0001, 0x3F);
    let src_lo = mem.read_ram(0x0002).wrapping_add(5);
    mem.write_ram(0x0002, src_lo);
    prep_ppu_palette_string(mem, &block[5..10]);
}

/// PrepPPUPaletteString 生产端（asm 内容锚 `PrepPPUPaletteString: ; 07:C37E` +
/// `WritePPUByte: ; 07:C36B`/`NextPPUByte: ; 07:C36E`/`EndVRAMStruct: ; 07:C376`/
/// `SeparateControlBits: ; 07:C3C6` 折叠——**线格式**（decomp A 级锚：`[hi][lo][ctrl|data…]`，
/// ctrl bit6=RLE、低 6 位=计数、$00=结束）。
///
/// **结构互核注**：EndVRAMStruct 的 `pla/pla` 逃逸=弹一层返回跳出写系——本模型折叠为
/// walk 终止（缓冲满 `cpx #_sizeof_VRAMStructBuffer`（$4F）路径=unported）；
/// 终止符写 buffer[x] **不推进索引**——下 call 从索引处覆写（双 call 连续生产语义自证）。
pub fn prep_ppu_palette_string(mem: &mut FrameMemory, src: &[u8]) {
    mem.write_ram(PPU_DATA_PENDING, 1);
    let mut x = mem.read_ram(VRAM_STRUCT_INDEX) as usize;
    let dst_lo = mem.read_ram(0x0000);
    let dst_hi = mem.read_ram(0x0001);
    let mut i = 0usize;
    loop {
        let ctrl = src[i];
        if ctrl == 0 {
            mem.write_ram(0x07A1 + x as u16, 0);
            return;
        }
        for b in [dst_hi, dst_lo, ctrl & 0xBF] {
            if x >= 0x4F {
                crate::unported!("bank07:C36E-full");
            }
            mem.write_ram(0x07A1 + x as u16, b);
            x += 1;
        }
        let count = (ctrl & 0x3F) as usize;
        let rle = ctrl & 0x40 != 0;
        if rle {
            let v = src[i + 1];
            for _ in 0..count {
                mem.write_ram(0x07A1 + x as u16, v);
                x += 1;
            }
            i += 2;
        } else {
            for k in 0..count {
                mem.write_ram(0x07A1 + x as u16, src[i + 1 + k]);
                x += 1;
            }
            i += 1 + count;
        }
        mem.write_ram(VRAM_STRUCT_INDEX, x as u8);
        // 结构互核：段间 iny 已含于分支步（非 RLE i+=1+count / RLE i+=2）——
        // 尾部不再 +1（初版双计越界案=相位误读族实例）
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 16 帧门（FrameCount&0xF≠0 ⇒ 静默）。
    #[test]
    fn star_palette_16frame_gate() {
        let mut m = FrameMemory::default();
        m.write_ram(FRAME_COUNT, 3);
        star_palette_switch(&mut m);
        assert_eq!(m.read_ram(INTRO_STAR_OFFSET), 0);
    }

    /// 队列门 + boot 路径全生产（双 call 缓冲几何+终止符覆写语义——结构互核注面）。
    #[test]
    fn star_palette_produces_two_stream_chunks() {
        let mut m = FrameMemory::default();
        m.write_ram(VRAM_STRUCT_INDEX, 1);
        star_palette_switch(&mut m);
        assert_eq!(m.read_ram(INTRO_STAR_OFFSET), 0);
        m.write_ram(VRAM_STRUCT_INDEX, 0);
        star_palette_switch(&mut m);
        let buf: Vec<u8> = (0..13).map(|k| m.read_ram(0x07A1 + k)).collect();
        // 段头序=[hi][lo]（asm `lda Temp00+1` 先写+decomp 大端址格式锚）
        assert_eq!(buf, vec![0x3F, 0x19, 0x03, 0x0F, 0x02, 0x13,
                             0x3F, 0x1D, 0x03, 0x00, 0x34, 0x0F, 0x00]);
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 12);
        assert_eq!(m.read_ram(PPU_DATA_PENDING), 1);
        assert_eq!(m.read_ram(INTRO_STAR_OFFSET), 1);
    }
}
