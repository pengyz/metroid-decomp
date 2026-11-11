//! 卡 C：PalettePtrTable 区域 bank 真项数=28（Palette00-1B）端口（TDD 锚）。
//!
//! asm 内容锚 `refs/m1disasm/SRC/prg2_norfair.asm:53-80`
//! `PalettePtrTable: ; 02:9560` 逐项 28 条（Palette00..1B），[28] 起是
//! `SpecItmsTblPtr: ; 02:9598`（下一张表，非本表指针）。
//! 标题 bank0（`refs/m1disasm/SRC/prg0_title.asm:3201`）只有 19 条（Palette00-12）。
//!
//! `SelectSamusPalette: ; 07:CB73`（`prg7_engine.asm:2535-2559`）对 JUSTIN BAILEY
//! （suitless, JIB=$69B3≠0）写 `PaletteDataPending = 2 + (m<<1|v) + 0x17`
//! （`adc #_id_Palette18-_id_Palette01.b`=0x17）⇒ 值 25..28 ⇒
//! `CheckPaletteWrite` 的 `dey/asl` 查 `PalettePtrTable[(pending-1)*2]` = 项 24..27
//! （Palette18..1B）——**全是合法项**。ROM bank2 实测：
//!   [24]=$A1FB stream `3f 10 04 0f 15 34 17 00`（Palette18）
//!   [25]=$A203 stream `3f 10 04 0f 15 34 19 00`（Palette19）
//!   [26]=$A20B stream `3f 10 04 0f 15 34 28 00`（Palette1A）
//!   [27]=$A213 stream `3f 10 04 0f 15 34 29 00`（Palette1B）
//!
//! 旧烘焙工具 `tools/bake_palette_data.py` 用 `N_ENTRIES=25` 截断了区域 bank，
//! rust 消费端 `nmi.rs::palette_write_pending` 又自设 `if idx >= 25` fail-loud 桩
//! ——asm `CheckPaletteWrite: ; 07:C1E0` 的 `@paletteDataIsPending: ; 07:C1FF`
//! 是 `dey/tya/asl/tay` **无界查表**，从不做范围检查。故 idx 25/27 触发
//! `unported!("bank07:C1FF-idx")` 导致 L4 f2390 格 blocked。

use game_core::frame::nmi::palette_write_pending;
use game_core::frame::FrameMemory;
use game_core::ram_map::{GAME_MODE, IN_AREA, PALETTE_PENDING};

/// 设 Norfair 区 bank（InArea=0x11 → bank2）、游戏内（GameMode=0）。
fn norfair_mem() -> FrameMemory {
    let mut m = FrameMemory::default();
    m.write_ram(GAME_MODE, 0x00);
    m.write_ram(IN_AREA, 0x11); // & 0x0F = 1 ⇒ bank 2（Norfair）
    m
}

#[test]
fn palette_ptr_table_idx25_writes_suitless_varia_palette19() {
    // pending=26 ⇒ idx 25 ⇒ Palette19（suitless varia）
    // ROM bank2 stream `3f 10 04 0f 15 34 19 00` ⇒ $3F10 ← 0F 15 34 19
    let mut m = norfair_mem();
    m.write_ram(PALETTE_PENDING, 26);
    palette_write_pending(&mut m);
    assert_eq!(
        &m.render.palette[16..20],
        &[0x0F, 0x15, 0x34, 0x19],
        "idx25=Palette19：$3F10..13 ← 0F 15 34 19（asm 28 项表）"
    );
    assert_eq!(m.read_ram(PALETTE_PENDING), 0, "消费后 PaletteDataPending 清零");
}

#[test]
fn palette_ptr_table_idx27_writes_suitless_varia_missile_palette1b() {
    // pending=28 ⇒ idx 27 ⇒ Palette1B（suitless varia + missiles）
    // ROM bank2 stream `3f 10 04 0f 15 34 29 00` ⇒ $3F10 ← 0F 15 34 29
    let mut m = norfair_mem();
    m.write_ram(PALETTE_PENDING, 28);
    palette_write_pending(&mut m);
    assert_eq!(
        &m.render.palette[16..20],
        &[0x0F, 0x15, 0x34, 0x29],
        "idx27=Palette1B：$3F10..13 ← 0F 15 34 29（asm 28 项表末项）"
    );
}

/// F1 整改 TDD（r9 证伪）：area→ROM bank 映射必须走 asm `BankTable 07:CA30`
/// 真值 `02 03 05 04 06`（=bank+1 形）⇒ area0..4 → bank [1,2,4,3,5]
/// （Brinstar=1/Norfair=2/Kraid=4/Tourian=3/Ridley=5——Kraid/Tourian 互换是
/// 旧式 `(InArea&0xF)+1` 的错）。ROM 逐字节自证：
/// `python3 -c "…off=16+7*0x4000+(0xCA30-0xC000)…"` → `02 03 05 04 06`。
/// 判别子：bank3(Tourian) Palette00 idx0 首 8B `3f00200f2016000f` vs
/// bank4(Kraid) `3f00200f2010000f`（[4]=0x16 vs 0x10 可判）。
#[test]
fn area_to_rom_bank_mapping_follows_bancctable() {
    // Kraid（InArea=0x12，&0x0F=2）必须读 bank4 而非 bank3。
    let mut m = FrameMemory::default();
    m.write_ram(GAME_MODE, 0x00);
    m.write_ram(IN_AREA, 0x12); // Kraid
    m.write_ram(PALETTE_PENDING, 1); // idx0 = Palette00
    palette_write_pending(&mut m);
    assert_eq!(
        m.render.palette[2], 0x10,
        "Kraid=area2→ROM bank4（BankTable $05-1）：Palette00 首块 data[2]=0x10；旧式误读 bank3=data[2]=0x16"
    );
    // Tourian（InArea=0x13，&0x0F=3）必须读 bank3 而非 bank4。
    let mut m = FrameMemory::default();
    m.write_ram(GAME_MODE, 0x00);
    m.write_ram(IN_AREA, 0x13); // Tourian
    m.write_ram(PALETTE_PENDING, 1);
    palette_write_pending(&mut m);
    assert_eq!(
        m.render.palette[2], 0x16,
        "Tourian=area3→ROM bank3（BankTable $04-1）：Palette00 首块 data[2]=0x16；旧式误读 bank4=data[2]=0x10"
    );
}
