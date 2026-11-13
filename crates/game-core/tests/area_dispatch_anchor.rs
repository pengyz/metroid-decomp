//! 卡 C piece④：per-area 图形表分发锚测（TDD 锚——防回退到单区硬编码）。
//!
//! asm 内容锚：
//! - `CopyAreaPointers: ; 07:C8B0`（prg7_engine.asm:1936）`lda AreaPointers_ROM,x`——
//!   ROM 侧=$959A 起 14B，取自**当前区 bank**（InArea 经 BankTable 07:CA30 换 bank）。
//! - `SetupRoom: ; 07:EA2B`（prg7_engine.asm:8578）`lda (AreaPointers_RAM.RoomPtrTable),y`
//!   经 $3B/$3C 间接查当前区 RoomPtrTable。
//! - `DrawObject: ; 07:EA60` `lda (AreaPointers_RAM.StructPtrTable),y` 经 $3D/$3E 间接。
//!
//! ROM 独立核验（file off=bank*0x4000+0x1560-0x4000+0x4000 区换算，python 逐字节）：
//! bank1(Brinstar) `14 a3 …` / bank2(Norfair) `1b a2 …` / bank4(Kraid) `d5 a1 …` /
//! bank3(Tourian) `d1 a7 …` / bank5(Ridley) `7f a1 …`。
//! 房流/结构/元 tile 烘焙件头注：Brinstar RoomPtrTable $A314(47 项)/StructPtrTable $A372(50)/
//! MetatileDefs $AEF0(260B)；Norfair $A21B(46)/$A277(49)/$AEEC(248B)。

use game_core::frame::mainloop::copy_area_pointers;
use game_core::frame::FrameMemory;
use game_core::ram_map::IN_AREA;

/// 首指针 lo/hi（ROM 五区 RoomPtrTable 域首项——bank*0x4000+0x1560 换算）。
const AREA0_PTR: (u8, u8) = (0x14, 0xA3); // Brinstar RoomPtrTable=$A314
const AREA1_PTR: (u8, u8) = (0x1B, 0xA2); // Norfair RoomPtrTable=$A21B
const AREA2_PTR: (u8, u8) = (0xD5, 0xA1); // Kraid RoomPtrTable=$A1D5
const AREA3_PTR: (u8, u8) = (0xD1, 0xA7); // Tourian RoomPtrTable=$A7D1
const AREA4_PTR: (u8, u8) = (0x7F, 0xA1); // Ridley RoomPtrTable=$A17F

#[test]
fn copy_area_pointers_dispatches_five_area_banks() {
    for (area, want) in [
        (0x00u8, AREA0_PTR),
        (0x11u8, AREA1_PTR), // InArea=$11（高位区码）&$0F=1——密码案 Norfair
        (0x12u8, AREA2_PTR),
        (0x13u8, AREA3_PTR),
        (0x14u8, AREA4_PTR),
    ] {
        let mut m = FrameMemory::default();
        m.write_ram(IN_AREA, area);
        copy_area_pointers(&mut m);
        let lo = m.read_ram(0x003B);
        let hi = m.read_ram(0x003C);
        assert_eq!(
            (lo, hi),
            want,
            "AreaPointers($3B/$3C) 首指针按 InArea={area:#04X} 选区（BankTable 07:CA30 序）"
        );
    }
}

#[test]
#[should_panic(expected = "unported @ bank07:959A-area")]
fn copy_area_pointers_unbaked_area_fails_loud() {
    // 未烘焙区（InArea&$0F=5+）——D7 fail-loud，确定性爆在 bank:addr。
    let mut m = FrameMemory::default();
    m.write_ram(IN_AREA, 0x05);
    copy_area_pointers(&mut m);
}

#[test]
fn room_graphics_dispatch_tables_match_rom_anchors() {
    // Brinstar（area0）：47 项房流，首项 $A441（RoomPtrTable $A314 lo/hi 对）。
    let b = game_data::assets::room_ptrs(0).expect("Brinstar 必烘焙");
    
    assert_eq!(
        game_data::assets::room_ptr(0, 0),
        Some(0xA441),
        "Brinstar 首房指针（提取自证 A441）"
    );
    assert_eq!(b.len(), 47 * 2, "47 项 ×2B");
    // Norfair（area1）：46 项房流，首项 $A3AB（$A21B lo/hi 对）。
    let n = game_data::assets::room_ptrs(1).expect("Norfair 必烘焙");
    
    assert_eq!(
        game_data::assets::room_ptr(1, 0),
        Some(0xA3AB),
        "Norfair 首房指针（ROM 核验 0xA3AB）"
    );
    assert_eq!(n.len(), 46 * 2, "46 项 ×2B");
    // 结构件：50/49 项。
    assert_eq!(game_data::assets::struct_ptrs(0).unwrap().len(), 50 * 2);
    assert_eq!(game_data::assets::struct_ptrs(1).unwrap().len(), 49 * 2);
    // 元 tile：260/248B，Norfair 首组 f1 f1 f1 f1（ROM $AEEC 核验）。
    assert_eq!(game_data::assets::metatile_defs(0).unwrap().len(), 260);
    let mt = game_data::assets::metatile_defs(1).unwrap();
    assert_eq!(mt.len(), 248);
    assert_eq!(&mt[0..4], &[0xF1, 0xF1, 0xF1, 0xF1], "Norfair MetatileDefs 首组（ROM $AEEC）");
    // 房流/结构：46/47 与 49/50 件。
    assert_eq!(game_data::assets::room_streams(1).unwrap().len(), 46);
    assert_eq!(game_data::assets::structures(1).unwrap().len(), 49);
}

#[test]
fn room_graphics_dispatch_unbaked_area_is_none() {
    // Kraid/Tourian/Ridley 未烘焙——None → 调用方 unported! fail-loud（D7）。
    assert!(game_data::assets::room_ptrs(2).is_none());
    assert!(game_data::assets::room_ptrs(3).is_none());
    assert!(game_data::assets::room_ptrs(4).is_none());
    assert!(game_data::assets::structures(5).is_none());
}

/// Norfair 五件套迁包锚：48/49/248 形状 + 房流 $FF 自界（原 `norfair_graphics.rs`
/// 自带锚测随文件删除，本测接管其判据——否则迁包会净降覆盖）。
#[test]
fn norfair_room_graphics_segments_match_rom_anchors() {
    assert_eq!(game_data::assets::room_ptrs(1).unwrap().len(), 46 * 2, "46 项×2B");
    assert_eq!(game_data::assets::struct_ptrs(1).unwrap().len(), 49 * 2, "49 项×2B");
    let mt = game_data::assets::metatile_defs(1).unwrap();
    assert_eq!(mt.len(), 248, "Norfair 元tile 248 B（$AEEC）");
    assert_eq!(mt.len() % 4, 0, "4B 组");
    let rooms = game_data::assets::room_streams(1).unwrap();
    assert_eq!(rooms.len(), 46, "46 房流");
    assert_eq!(*rooms[0].last().unwrap(), 0xFF, "房流 $FF 自界");
    let st = game_data::assets::structures(1).unwrap();
    assert_eq!(st.len(), 49, "49 结构体");
    assert_eq!(*st[0].last().unwrap(), 0xFF, "结构 $FF 自界");
    // 起始房 9 的指针（ROM 核验口径，与 Brinstar 同形锚）
    assert_eq!(game_data::assets::room_ptr(1, 9), Some(0xA55F), "Norfair 起始房 9 指针");
}
