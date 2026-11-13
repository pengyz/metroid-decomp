//! GFX 加载族（bank07:C5D7 族；two-layer 域模块——frame/ 互斥面）。
//!
//! **asm 内容锚+结构互核注**（施工令②精化版）——
//! refs/m1disasm/SRC/prg7_engine.asm `InitTitleGFX: ; 07:C5D7`（载入序列：Title→Samus→
//! [SamusSuitless]（`lda JustInBailey/beq` 门）→IntroSprites→Font_Complete→Font_Hud→
//! Solid_0FC0→Solid_1FC0（尾 `jmp LoadGFX`））+ `LoadGFX: ; 07:C7AB`（描述子装载：
//! 7*y+6 偏移→**GFXInfo 7B entry→$00-$06**（倒序拷贝）→`jsr BankSwitch`（$00=bank/
//! $28=($28&0x18)|bank/PRG 串行）→`$FF&=!INCR_DOWN`→`jsr CopyGFXBlock`（CHR 写——渲染
//! 面）→`ldy CurrentBank/jmp BankSwitch`（回切））。
//!
//! **结构互核注**：①GFXInfo 7B 描述子=bank+src(2)+dest(2)+size(2)（宏体
//! `GFXInfoEntry args gfx, dest`：`byte bank(gfx)/PtrTableEntry/word dest/word size` 自证）
//! ②BankSwitch **双切**（进/出）——$00 终态=CurrentBank（回切覆盖 entry[0]）③$FF 增量
//! 位清（`and #~PPUCTRL_INCR_DOWN`）——RAM watch 面。
//! **数据岛边界**：GFXInfo 8 件（ROM 提取自证 prg[0x1C6E0+idx*7]）。
//!
//! 生效面：CopyGFXBlock 的 CHR 写=PPU 面（write_io 族——渲染面挂 task-13）。

use super::memory::FrameMemory;
use crate::ram_map::{CURRENT_BANK, JUST_IN_BAILEY, MMC1CTRL_ZP, PPUCTRL_ZP, SWITCH_UPPER};

/// GFXInfo 描述子 id（表序自证——macros.asm PtrTableEntry 序号+prg7_engine 注释互证）。
/// 描述子真值=prg[0x1C6E0+id*7]（**ROM 提取自证**——tools/bake_gfx_blocks.py 提取径同源，
/// --check 校验径可复跑）。
pub mod gfx_id {
    pub const SAMUS: u8 = 0;
    // Brinstar 背景/精灵件（InitBrinstarGFX 消费——asm :1523 `InitBrinstarGFX: ; 07:C604`）
    pub const BRIN_BG1: u8 = 3;
    pub const BRINSTAR_SPRITES: u8 = 6;
    // 区域背景/精灵件（InitNorfairGFX/Tourian/Kraid/Ridley 消费；r25 批次 3 补烘焙）
    pub const CREBG1: u8 = 4;
    pub const CREBG2: u8 = 5;
    pub const NORF_BG1: u8 = 7;
    pub const NORF_BG2: u8 = 8;
    pub const NORFAIR_SPRITES: u8 = 9;
    pub const INTRO_SPRITES: u8 = 20;
    pub const TITLE: u8 = 21;
    pub const SOLID_1FC0: u8 = 22;
    pub const FONT_COMPLETE: u8 = 23;
    pub const FONT_HUD: u8 = 24;
    pub const SOLID_0FC0: u8 = 25;
    pub const SAMUS_SUITLESS: u8 = 27;
}

/// GFXInfo 描述子读取（7B=bank/src2/dest2/size2）。
///
/// **机制形（去重修）**：asm `LoadGFX: ; 07:C7AB` 走 `ldy #7*id / lda GFXInfo,y..` 表间接读
/// （表址 prg7_engine.asm:1669 `GFXInfo ; 07:C6E0`），rust 侧对应
/// `game_data::assets::gfx_info()`（ROM 29 项×7B 全表烘焙，`tools/bake_gfx.py` 生成，
/// 已核 `GFX_INFO == ROM[0x1C6E0..+203]` 逐字节）。旧形是手写 13 条 match 折叠——
/// 表值虽同，但机制是"编译器常量"而非"表间接读"，且与数据面重复烘焙两份。
///
/// 越界（id>=29）保持 fail-loud（D7）：ROM 表只有 29 项，读出去就是未取证区。
pub fn gfx_info(id: u8) -> [u8; 7] {
    let info = game_data::assets::gfx_info();
    let off = id as usize * 7;
    if off + 7 > info.len() {
        crate::unported!("bank07:C6E0-entry"); // ROM 表 29 项外（表尾）
    }
    let mut e = [0u8; 7];
    e.copy_from_slice(&info[off..off + 7]);
    e
}

/// InitNorfairGFX（**asm 内容锚** `InitNorfairGFX: ; 07:C622`——prg7_engine.asm:1542-1561）：
/// `ldy #_id_GFX_CREBG1 / jsr LoadGFX` → CREBG2 → NorfBG1 → NorfBG2 → NorfairSprites →
/// Solid_0FC0 → `ldy #_id_GFX_Solid_1FC0 / jmp LoadGFX`（尾跳，7 件）。
pub fn init_norfair_gfx(mem: &mut FrameMemory) {
    use gfx_id::*;
    for id in [CREBG1, CREBG2, NORF_BG1, NORF_BG2, NORFAIR_SPRITES, SOLID_0FC0, SOLID_1FC0] {
        load_gfx(mem, id);
    }
}

/// LoadSamusGFX 单元（07:C5DC，prg7_engine.asm:1496-1520）：密码进场序载入。
/// asm 全形：`ldy #GFX_Samus/jsr LoadGFX` → `lda JustInBailey/beq @endIf_A`→
/// `ldy #GFX_SamusSuitless/jsr LoadGFX` → IntroSprites/Font_Complete/Font_Hud/
/// Solid_0FC0 → `ldy #GFX_Solid_1FC0/jmp LoadGFX`（尾跳）。**不含 TITLE**（区别 init_title_gfx）。
pub fn load_samus_gfx(mem: &mut FrameMemory) {
    use gfx_id::*;
    let mut loads = vec![SAMUS];
    if mem.read_wram(JUST_IN_BAILEY) != 0 {
        loads.push(SAMUS_SUITLESS);
    }
    loads.extend_from_slice(&[INTRO_SPRITES, FONT_COMPLETE, FONT_HUD, SOLID_0FC0, SOLID_1FC0]);
    for id in loads {
        load_gfx(mem, id);
    }
}

/// InitTitleGFX 单元（boot 序：7 件载入；JustInBailey=0 时跳 Suitless）。
pub fn init_title_gfx(mem: &mut FrameMemory) {
    use gfx_id::*;
    let mut loads = vec![TITLE, SAMUS];
    if mem.read_wram(JUST_IN_BAILEY) != 0 {
        loads.push(SAMUS_SUITLESS);
    }
    loads.extend_from_slice(&[INTRO_SPRITES, FONT_COMPLETE, FONT_HUD, SOLID_0FC0, SOLID_1FC0]);
    for id in loads {
        load_gfx(mem, id);
    }
}

/// GFX 块源面（CopyGFXBlock ROM 侧）。
///
/// **机制形（去重修）**：asm `CopyGFXBlock: ; 07:C7D5` 从描述子给的 ROM 地址逐字节搬到 PPU；
/// rust 侧对应 `assets::gfx_data()`（29 项数据体拼接，`GFX_OFFS[id]=(偏移,尺寸)` 索引），
/// 已核 **29 项全部 `GFX_DATA[off..off+size] == ROM[bank*0x4000+src-0x8000 ..+size]` 逐字节**。
/// 旧形是 13 条 `&gb::X_GFX` match + 独立生成件 `gfx_blocks.rs`（第三份重复烘焙）。
///
/// 越界（id>=29）保持 fail-loud；32 位平台对 u32 偏移的安全边界由 `GFX_OFFS` 自身保证
/// （生成期已校验 `off+size <= GFX_DATA.len()`）。
pub fn gfx_block(id: u8) -> &'static [u8] {
    let Some((off, size)) = game_data::assets::gfx_offs(id as usize) else {
        crate::unported!("bank07:C6E0-entry"); // ROM 表 29 项外（表尾）
    };
    let data = game_data::assets::gfx_data();
    debug_assert!(off + size <= data.len(), "gfx_offs 越界（描述子应自洽）");
    &data[off..off + size]
}

/// LoadGFX 折叠（描述子→$00-$06/BankSwitch 双切/$FF 位——CHR 写=渲染面 write_io 注）。
pub fn load_gfx(mem: &mut FrameMemory, id: u8) {
    let e = gfx_info(id);
    // 描述子倒序拷贝（LC7B6：x=6..0 ← entry[7id+6..7id]）
    for x in 0..7u16 {
        mem.write_ram(x, e[x as usize]);
    }
    // BankSwitch 进切（C4EF 族：$00=bank/$28=($28&0x18)|bank/PRG 串行）
    bank_switch(mem, e[0]);
    // $FF 增量位清（`and #~PPUCTRL_INCR_DOWN`）
    let f = mem.read_ram(PPUCTRL_ZP) & !0x04;
    mem.write_ram(PPUCTRL_ZP, f);
    mem.write_io(0x2000, f);
    // CopyGFXBlock（bank07:C7D5 asm 引证主体）：`lda (Temp01_GFXSource),y / sta PPUDATA`
    // 逐字节 ROM→pattern table；256B 步进 inc src/dest 高字节；`size_lo==0` 预减 quirk
    // 净效果=恰 size 字节；`dec Temp05_GFXSize` 消耗环终态 $05/$06=0/0。
    // ROM 侧=game-data 烘焙块（src/size 同一描述子提取，--check 校验径）。
    let block = gfx_block(id);
    let size = u16::from_le_bytes([e[5], e[6]]) as usize;
    debug_assert_eq!(block.len(), size, "烘焙块长与描述子 size 不一致（bake --check 应拦）");
    let dest = u16::from_le_bytes([e[3], e[4]]);
    mem.write_io(0x2006, (dest >> 8) as u8); // PPUADDR hi
    mem.write_io(0x2006, dest as u8); // PPUADDR lo
    for &b in block.iter().take(size) {
        mem.write_io(0x2007, b); // PPUDATA（inc=1——$FF 增量位已清）
    }
    mem.write_ram(0x0005, 0);
    mem.write_ram(0x0006, 0);
    // 回切（CurrentBank）
    let cur = mem.read_ram(CURRENT_BANK);
    bank_switch(mem, cur);
}

/// BankSwitch（C4EF 族共享段——与 c4de_unit 同源）：$00=bank/$28 组合/PRG 串行。
fn bank_switch(mem: &mut FrameMemory, bank: u8) {
    mem.write_ram(0x0000, bank);
    let v = (mem.read_ram(SWITCH_UPPER) & 0x18) | bank;
    mem.write_ram(SWITCH_UPPER, v);
    mem.regs.a = v;
    super::boot::mmc1_serial_write(mem, super::boot::io::MMC1_PRG);
    let _ = MMC1CTRL_ZP;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// InitTitleGFX boot 面：末件 Solid_1FC0 描述子终态（$00=回切 CurrentBank=0/
    /// $01-$06=entry22[1..6]）+$FF 增量位清。
    #[test]
    fn init_title_gfx_boot_final_descriptor() {
        let mut m = FrameMemory::default();
        init_title_gfx(&mut m);
        assert_eq!(m.read_ram(0x0000), 0); // 回切 CurrentBank=0
        assert_eq!(
            [m.read_ram(1), m.read_ram(2), m.read_ram(3), m.read_ram(4), m.read_ram(5), m.read_ram(6)],
            [0x80, 0x99, 0xC0, 0x1F, 0x00, 0x00] // entry22 src/dest/**size 消耗归零**（B 案修）
        );
        assert_eq!(m.read_ram(PPUCTRL_ZP) & 0x04, 0); // 增量位清
    }

    /// GFXInfo 表 8 项钉死（**独立对 ROM**——prg[0x1C6E0+id*7] 提取值，tools/bake_gfx_blocks.py
    /// --check 同源核）；防同源弱化（A7）：表值损坏必红。
    #[test]
    fn pin_gfx_info_table_rom_truth() {
        // (id, bank, src, dest, size)——ROM 提取自证（2026-10-23 实测原文）
        let truth: [(u8, u8, u16, u16, u16); 8] = [
            (0, 6, 0x8000, 0x0000, 0x09A0),
            (20, 6, 0x89A0, 0x0C00, 0x0100),
            (21, 6, 0x8BE0, 0x1400, 0x0500),
            (22, 6, 0x9980, 0x1FC0, 0x0040),
            (23, 6, 0xB4C0, 0x1000, 0x0400),
            (24, 6, 0xB4C0, 0x0A00, 0x00A0),
            (25, 6, 0x9980, 0x0FC0, 0x0040),
            (27, 6, 0x90E0, 0x0000, 0x07B0),
        ];
        for (id, bank, src, dest, size) in truth {
            let e = gfx_info(id);
            assert_eq!(e[0], bank, "id{id} bank");
            assert_eq!(u16::from_le_bytes([e[1], e[2]]), src, "id{id} src");
            assert_eq!(u16::from_le_bytes([e[3], e[4]]), dest, "id{id} dest");
            assert_eq!(u16::from_le_bytes([e[5], e[6]]), size, "id{id} size");
            assert_eq!(gfx_block(id).len() as u16, size, "id{id} 烘焙块长==描述子 size");
        }
    }

    /// CopyGFXBlock CHR 写面（task-13 首件）：load_gfx(TITLE) ⇒ chr[$1400..$1900]==烘焙块
    /// （全逐字节）+ 邻域不越界（$13FF/$1900 不动）+ $05/$06 消耗终态 0/0。
    #[test]
    fn copy_gfx_block_fills_chr_via_ppu() {
        let mut m = FrameMemory::default();
        load_gfx(&mut m, 21); // TITLE（dest=$1400 size=$0500）
        let block = gfx_block(21);
        assert_eq!(&m.render.chr[0x1400..0x1400 + block.len()], block, "CHR 载面全逐字节");
        assert_eq!(m.render.chr[0x13FF], 0x00, "下界不越界");
        assert_eq!(m.render.chr[0x1400 + block.len()], 0x00, "上界不越界");
        assert_eq!((m.read_ram(0x0005), m.read_ram(0x0006)), (0, 0), "size 消耗终态");
    }

    /// InitTitleGFX 七件载面（boot 序 JustInBailey=0）：各块 dest 区全对 + 无重叠互污。
    #[test]
    fn init_title_gfx_loads_all_blocks() {
        let mut m = FrameMemory::default();
        init_title_gfx(&mut m);
        for id in [21u8, 0, 20, 23, 24, 25, 22] {
            let e = gfx_info(id);
            let dest = u16::from_le_bytes([e[3], e[4]]) as usize;
            let block = gfx_block(id);
            assert_eq!(&m.render.chr[dest..dest + block.len()], block, "id{id} dest=${dest:04X} 载面");
        }
    }
}
