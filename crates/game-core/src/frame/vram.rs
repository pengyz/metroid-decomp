//! VRAMStruct 消费端（PPU 写面——asm 引证主体，链规 17）。
//!
//! refs 考证（bank:addr（A 级原文））：
//! - `PreparePPUProcess: ; 07:C20E`（prg7_engine.asm:551——ptr 装载→`jmp VRAMStructWrite`）
//! - `VRAMStructWrite: ; 07:C30C`（:856——每记录尾 fallthrough 回读下一记录 hi，0⇒`jmp WriteScroll`）
//! - `VRAMStructWriteData: ; 07:C2E4`（:813——**逐字节语义**）：
//!   `sta PPUADDR`（hi）→`iny/lda (ptr),y/sta PPUADDR`（lo）→`iny/lda ctrl`
//!   →`asl`（CF=bit7→`jsr SetPPUInc`（07:C318——PPUCTRL_ZP/PPUCTRL 增量 1/32））
//!   →`asl`（CF=bit6=RLE）→`lda ctrl/and #$3F/tax`（计数；**0⇒256**（dex/bne 环））
//!   → RLE：`iny` 至数据字节，环内不 iny；非 RLE：环内逐字节 `iny`
//!   → `lda (ptr),y / sta PPUDATA` ×count →`iny`→`jsr AddYToPtr00`（ptr+=Y）。
//! - 记录布局=macros.asm `VRAMStructData`/`VRAMStructDataRepeat`/`VRAMStructEnd`（.byte $00）。
//!
//! 数据面=game-data 烘焙（tools/bake_vram_structs.py，--check 校验径）。

use super::memory::FrameMemory;

/// VRAMStructWrite 消费（blob 含 hi==0 终止符——消费至终止）。
/// 写径=write_io($2006/$2007)（RenderState 三分域）；增量由 SetPPUInc 面（$FF bit2）进 write_io 取。
pub fn vram_struct_write(mem: &mut FrameMemory, blob: &[u8]) {
    // asm 入口：`ldx PPUSTATUS`（07:C30C）——地址锁存复位
    mem.render.reset_addr_latch();
    let mut i = 0usize;
    loop {
        let hi = blob[i];
        if hi == 0 {
            return; // VRAMStructEnd ⇒ jmp WriteScroll（滚动恢复——渲染面不建模）
        }
        let lo = blob[i + 1];
        let ctrl = blob[i + 2];
        // SetPPUInc（07:C318）：CF=ctrl bit7 ⇒ 增量 32/1（PPUCTRL_ZP bit2 + PPUCTRL 面）
        let inc_down = ctrl & 0x80 != 0;
        let ff = mem.read_ram(crate::ram_map::PPUCTRL_ZP);
        let ff = if inc_down { ff | 0x04 } else { ff & !0x04 };
        mem.write_ram(crate::ram_map::PPUCTRL_ZP, ff);
        mem.write_io(super::boot::io::PPU_CTRL, ff);
        let rle = ctrl & 0x40 != 0;
        let count = match ctrl & 0x3F {
            0 => 256usize, // `and #$3F; tax`+`dex/bne`：0⇒256 次（字节驱动 quirk）
            n => n as usize,
        };
        mem.write_io(0x2006, hi);
        mem.write_io(0x2006, lo);
        if rle {
            let b = blob[i + 3];
            for _ in 0..count {
                mem.write_io(0x2007, b);
            }
            i += 4;
        } else {
            for k in 0..count {
                mem.write_io(0x2007, blob[i + 3 + k]);
            }
            i += 3 + count;
        }
    }
}

/// 单名表清屏 PPU 面（asm 引证主体 `ClearNameTable: ; 07:C175`——prg7_engine.asm:395）：
/// `ldx PPUSTATUS`（锁存复位）→增量 1→`HiPPUTable[nt]→$2006/#$00→$2006`→
/// `ldx #$04/ldy #$00` 环 `sta PPUDATA`——**NT+属性 1024B**（4×256；:385「also clears
/// the associated attribute table」自证）填 fill。nt=名表号 0..3（HiPPUTable $C19F=20/24/28/2C）。
pub fn clear_name_table_ppu(mem: &mut FrameMemory, nt: u8, fill: u8) {
    mem.render.reset_addr_latch();
    // PPUCTRL_ZP/PPUCTRL 增量 1（`and #~PPUCTRL_INCR_DOWN`）
    let ff = mem.read_ram(crate::ram_map::PPUCTRL_ZP) & !0x04;
    mem.write_ram(crate::ram_map::PPUCTRL_ZP, ff);
    mem.write_io(super::boot::io::PPU_CTRL, ff);
    let base: u16 = 0x2000 + 0x400 * (nt as u16 & 3); // HiPPUTable[nt]=20/24/28/2C
    mem.write_io(0x2006, (base >> 8) as u8);
    mem.write_io(0x2006, 0x00);
    for _ in 0..1024 {
        mem.write_io(0x2007, fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 非 RLE 记录（hi/lo/len/data×len）→ 名表写+终止符停。
    #[test]
    fn plain_record_writes_nametable() {
        let mut m = FrameMemory::default();
        // $2000 起 3 字节 AB CD EF + 终止
        let blob = [0x20, 0x00, 0x03, 0xAB, 0xCD, 0xEF, 0x00];
        vram_struct_write(&mut m, &blob);
        assert_eq!(m.render.nametable[0..3], [0xAB, 0xCD, 0xEF]);
        assert_eq!(m.render.nametable[3], 0x00, "终止后不写");
        assert_eq!(m.render.ppuaddr, 0x2003, "ppuaddr 终态=起点+count（inc=1）");
    }

    /// RLE 记录（ctrl|0x40）：单字节重复 count 次。
    #[test]
    fn rle_record_repeats_byte() {
        let mut m = FrameMemory::default();
        let blob = [0x20, 0x00, 0x40 | 0x04, 0x77, 0x00]; // 0x77 ×4 @ $2000
        vram_struct_write(&mut m, &blob);
        assert_eq!(m.render.nametable[0..4], [0x77; 4]);
    }

    /// 计数 quirk（低6位=0 ⇒ 256）+ 多记录链 + 增量 32 位（ctrl bit7）。
    #[test]
    fn count_zero_means_256_and_inc32_flag() {
        let mut m = FrameMemory::default();
        // 记录1：RLE 0x55×256 @ $2100（ctrl=0x40 低6位 0）；记录2：inc32 非 RLE 2B @ $23C0；终止
        let mut blob = vec![0x21, 0x00, 0x40, 0x55];
        blob.extend([0x23, 0xC0, 0x80 | 0x02, 0x11, 0x22]);
        blob.push(0x00);
        vram_struct_write(&mut m, &blob);
        assert_eq!(m.render.nametable[0x100..0x200], [0x55; 256], "RLE 256 面");
        // $23C0=属性表区（nametable vec $2000-$23FF 域内偏移 0x3C0）
        assert_eq!(m.render.nametable[0x3C0], 0x11);
        assert_eq!(m.render.nametable[0x3E0], 0x22, "inc=32 步进（bit7 旗）——次字节落 +32");
    }

    /// intro BG 真值回归（烘焙 blob 首记录=属性表 $23C0 32B——game-data 同源对拍）。
    #[test]
    fn intro_bg_blob_first_record_regression() {
        let mut m = FrameMemory::default();
        let blob: &[u8] = &game_data::title::vram_structs::INTRO_BG_VRAM_STRUCT;
        assert_eq!(*blob.last().unwrap(), 0x00, "含 VRAMStructEnd 终止");
        vram_struct_write(&mut m, blob);
        // 首记录：$23C0 属性表 32B（前 8B=00——asm :620 自证）
        assert_eq!(m.render.nametable[0x3C0..0x3C8], [0u8; 8], "attr 首 8B=00（asm 对）");
        assert_eq!(m.render.nametable[0x3C8], 0xFF, "attr 次 8B 起=FF（asm 对）");
    }
}
