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
pub use game_data::render::render_data::INTRO_STAR_PALETTES;

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

    /// 【复审残余 4】终止符断言独立案（review 4 案之 4——覆盖恢复）。
    #[test]
    fn terminator_byte_assertion_standalone() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0000, 0x00);
        m.write_ram(0x0001, 0x20);
        write_vram_string(&mut m, &[0x22, 0x11, 0x22, 0x33, 0x44]); // 2×2 块
        // 段末（x=10）=终止符 0（EndVRAMStruct 契约——索引不动）
        assert_eq!(m.read_ram(0x07A1 + 10), 0x00);
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 10); // 索引=段末（终止符不计）
    }

    /// 【返工补案 1】单行块（宽=3|高=1）+**终止符断言**（F1——+末=0 且索引不动）。
    #[test]
    fn write_vram_string_single_row_terminator() {
        let mut m = FrameMemory::default();
        write_vram_string(&mut m, &[0x13, 0xAA, 0xBB, 0xCC]); // 高1|宽3
        let buf: Vec<u8> = (0..8).map(|k| m.read_ram(0x07A1 + k)).collect();
        // [00][00][03][AA][BB][CC][00 终止][—]
        assert_eq!(&buf[..7], &[0x00, 0x00, 0x03, 0xAA, 0xBB, 0xCC, 0x00]);
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 6); // 索引=段末（终止符不计）
    }

    /// 【返工补案 2】1×N 竖排（宽=1|高=3）——行间换行几何。
    #[test]
    fn write_vram_string_single_col() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0000, 0x00);
        m.write_ram(0x0001, 0x20);
        write_vram_string(&mut m, &[0x31, 0xAA, 0xBB, 0xCC]); // 高3|宽1
        let buf: Vec<u8> = (0..12).map(|k| m.read_ram(0x07A1 + k)).collect();
        assert_eq!(&buf[..12], &[0x20, 0x00, 0x01, 0xAA,
                                0x20, 0x20, 0x01, 0xBB,
                                0x20, 0x40, 0x01, 0xCC]);
    }

    /// 【返工补案 3】恰满边界（F2——asm 精确序：写 79B 后溢出=0x4E 后中止）。
    #[test]
    fn write_vram_string_exact_full_boundary() {
        let mut m = FrameMemory::default();
        // 高1|宽16+高1|宽16…：构造恰 79B 场景——宽16|高2 行 chunk=2*(3+16)=38B（不足）
        // 直接验共享件：写 79 字节成功、第 80 触发溢出域
        let mut x = 0usize;
        // asm 精确序：写 0..0x4E（79B）→第 79 写后 inx=$4F⇒溢出信号（字节已落 0x4E）
        for i in 0..78u8 {
            match write_ppu_byte_to_buffer(&mut m, x, i) {
                Some(nx) => x = nx,
                None => panic!("78B 内不应溢出（i={i}）"),
            }
        }
        assert_eq!(x, 0x4E);
        assert!(write_ppu_byte_to_buffer(&mut m, x, 0x4E).is_none()); // 第 79 写→溢出信号
        assert_eq!(m.read_ram(0x07A1 + 0x4E), 0x4E); // 字节已落（asm 先写后检）
    }

    /// 【溢出路补钉】（port2 字节对表）：溢出⇒终止写在**重载 committed 索引**
    /// （非溢出点）——丢弃未提交段语义。
    #[test]
    fn overflow_terminator_at_committed_index() {
        let mut m = FrameMemory::default();
        // committed 索引=3（模拟已提交段末）
        m.write_ram(VRAM_STRUCT_INDEX, 3);
        // 走 x=0..0x4E 连写（溢出第 79 写）——committed 索引仍=3
        let mut x = 0usize;
        let mut overflowed = false;
        for i in 0..200u8 {
            match write_ppu_byte_to_buffer(&mut m, x, i | 0x80) {
                Some(nx) => x = nx,
                None => { overflowed = true; break; }
            }
        }
        assert!(overflowed);
        assert_eq!(m.read_ram(0x07A1 + 3), 0x00); // **终止在重载索引 3**（非 0x4E）
        assert_eq!(m.read_ram(0x07A1 + 0x4E) & 0x80, 0x80); // 溢出点字节已落（先写后检）
    }

    /// 【返工补案 4】终止符覆盖语义（F1——双 call 连续：终止符被下 call 覆写）。
    #[test]
    fn end_vram_struct_terminator_overwrite() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0000, 0x00);
        m.write_ram(0x0001, 0x21); // dst=$2100（hi≠0）
        write_vram_string(&mut m, &[0x11, 0xAA]); // 单块 4B（[21][00][01][AA]）
        assert_eq!(m.read_ram(0x07A1 + 4), 0x00); // 终止符在 +4（索引=4 不动语义）
        // 下 call 从同索引覆写（终止符位置被新块头 hi 覆写）
        m.write_ram(0x0000, 0x00);
        m.write_ram(0x0001, 0x21);
        write_vram_string(&mut m, &[0x11, 0xBB]);
        assert_eq!(m.read_ram(0x07A1 + 4), 0x21); // 新块头 hi 覆写旧终止符
    }

    /// WriteVRAMString 块写手（段头三字节+名表换行 +$20——S6-1 面）。
    #[test]
    fn write_vram_string_blocks() {
        let mut m = FrameMemory::default();
        m.write_ram(0x0000, 0x00); // dst lo
        m.write_ram(0x0001, 0x20); // dst hi（$2000）
        // 宽2|高2 打包=0x22；瓦数据 4 字节
        let src = [0x22u8, 0xAA, 0xBB, 0xCC, 0xDD];
        write_vram_string(&mut m, &src);
        // 行1：[20][00][02][AA][BB]；行2：[20][20][02][CC][DD]
        let buf: Vec<u8> = (0..10).map(|k| m.read_ram(0x07A1 + k)).collect();
        assert_eq!(buf, vec![0x20, 0x00, 0x02, 0xAA, 0xBB,
                             0x20, 0x20, 0x02, 0xCC, 0xDD]);
        assert_eq!(m.read_ram(VRAM_STRUCT_INDEX), 10);
        assert_eq!(m.read_ram(0x07A1 + 10), 0x00); // **终止符断言**（F1 遮蔽修复）
        assert_eq!(m.read_ram(0x0001), 0x20); // 尾指针 $2040
        assert_eq!(m.read_ram(0x0000), 0x40);
    }

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

/// WritePPUByte 共享件（**asm 精确序**——F2 边界契约）：`sta VRAMStructBuffer,x/
/// inx/cpx #$4F/bcc @RTS`——**先写后检**：写 x=0..0x4E（79B）→inx 至 $4F⇒溢出。
/// 返回 Some(新 x) 或 None（溢出——调用方走 EndVRAMStruct 语义：补 0 终止+中止）。
fn write_ppu_byte_to_buffer(mem: &mut FrameMemory, x: usize, byte: u8) -> Option<usize> {
    mem.write_ram(0x07A1 + x as u16, byte);
    let x = x + 1;
    if x < 0x4F {
        Some(x)
    } else {
        // **溢出路**（port2 字节对表——`ldx $07A0`（重载 committed 索引）/`lda #0/
        // sta $07A1,x`（终止写在**重载索引**非溢出点）/`pla pla`（双 PLA 断链））。
        // 语义=**丢弃未提交段**（终止落上次 stx 索引处）。
        let committed = mem.read_ram(VRAM_STRUCT_INDEX) as u16;
        mem.write_ram(0x07A1 + committed, 0);
        None // 调用方返回（双 PLA 断链域形=中止）
    }
}

/// EndVRAMStruct 终止件（asm 内容锚 `EndVRAMStruct: ; 07:C376`）：`lda #$00/sta
/// VRAMStructBuffer,x`（终止符——**索引不动**）+溢出安全（PLA PLA 逃逸域形=中止）。
fn end_vram_struct(mem: &mut FrameMemory, x: usize) {
    if x < 0x4F {
        mem.write_ram(0x07A1 + x as u16, 0);
    }
    // 索引不推进（终止符在当前 x——下 call 覆写语义）
}

/// WriteVRAMString（**asm 内容锚**——S6-1 生产者族；task-42 spec §2.1）：
/// `WriteVRAMString: ; 07:C328`（`PPUDataPending←1`/宽=src[0]&$0F/高=src[0]>>4
/// （Adiv16）/@loop_row：`[hi][lo][宽]` 段头三字节→@loop_tile：瓦数据逐字 WritePPUByte
/// →`stx index`/`AddYToPtr00 +$20`（**名表换行**（32 增量））/高--/环）。
///
/// **结构互核注**：行间 +$20=名表换行语义（spec §2.1 自证）；段头宽=长度（解码器
/// 同构消费——[hi][lo][len][tile…] 行 chunk）；src 格式=[宽|高 打包字节][瓦数据...]。
pub fn write_vram_string(mem: &mut FrameMemory, src: &[u8]) {
    mem.write_ram(PPU_DATA_PENDING, 1);
    let packed = src[0];
    let width = (packed & 0x0F) as usize;
    let height = (packed >> 4) as usize;
    let mut x = mem.read_ram(VRAM_STRUCT_INDEX) as usize;
    let mut dst = (mem.read_ram(0x0001) as u16) << 8 | mem.read_ram(0x0000) as u16;
    let mut yi = 1usize;
    let mut overflowed = false;
    for _row in 0..height {
        let hi = (dst >> 8) as u8;
        let lo = dst as u8;
        for b in [hi, lo, width as u8] {
            match write_ppu_byte_to_buffer(mem, x, b) {
                Some(nx) => x = nx,
                None => { overflowed = true; break; }
            }
        }
        if overflowed { break; }
        for _ in 0..width {
            let t = src.get(yi).copied().unwrap_or(0);
            yi += 1;
            match write_ppu_byte_to_buffer(mem, x, t) {
                Some(nx) => x = nx,
                None => { overflowed = true; break; }
            }
        }
        if overflowed { break; }
        mem.write_ram(VRAM_STRUCT_INDEX, x as u8);
        dst = dst.wrapping_add(0x20);
    }
    // **F1：正常路径 EndVRAMStruct 终止符**（asm C376 契约——0+$7A1,X 索引不动）
    end_vram_struct(mem, x);
    mem.write_ram(0x0000, dst as u8);
    mem.write_ram(0x0001, (dst >> 8) as u8);
}
